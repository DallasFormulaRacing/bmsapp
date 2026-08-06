use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, broadcast};
use tokio::time;
use tokio_tungstenite::accept_async;

mod bridge;
mod can;
mod device;
mod websocket;
#[cfg(target_os = "linux")]
use can::socket::CanSocket;
use can::{CanCommand, CanNode, DaqCanCommand, DfrCanId, DfrCanMessageBuf};
use device::DeviceRegistry;
use websocket::{
    BackendEvent, BackendEventData, backend_event, decode_incoming, encode_outgoing,
};
const SERVER_ADDR: &str = "0.0.0.0:9002";
const DEVICE_STATUS_BROADCAST_INTERVAL: Duration = Duration::from_secs(1);
const DEVICE_OFFLINE_TIMEOUT: Duration = Duration::from_secs(3);
const DEVICE_TIMEOUT_CHECK_INTERVAL: Duration = Duration::from_millis(500);
/// Raspi -> BMS telemetry poll period (pack + 6 cell segments + 2 therm halves).
const BMS_TELEMETRY_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[cfg(not(target_os = "linux"))]
#[derive(Clone)]
struct CanSocket;

#[cfg(not(target_os = "linux"))]
impl CanSocket {
    fn open(_interface: &str) -> Result<Self, std::io::Error> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "CAN sockets are only supported on Linux",
        ))
    }

    async fn read_message(&self) -> Result<Option<DfrCanMessageBuf>, std::io::Error> {
        Ok(None)
    }

    async fn write_message(&self, _message: &DfrCanMessageBuf) -> Result<(), std::io::Error> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "CAN sockets are only supported on Linux",
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(SERVER_ADDR).await?;
    println!("can-bridge listening on ws://{SERVER_ADDR}");
    let registry = Arc::new(Mutex::new(DeviceRegistry::new()));
    let can_socket = open_can_socket();
    let (event_tx, _) = broadcast::channel::<BackendEvent>(128);

    if let Some(socket) = can_socket.clone() {
        tokio::spawn(can_receive_task(
            socket.clone(),
            Arc::clone(&registry),
            event_tx.clone(),
        ));

        // BMS GUI path: request/response - poll pack/cells/temps; BMS replies with same cmds.
        spawn_bms_polling_task(socket.clone(), event_tx.clone());
        println!(
            "BMS telemetry polling every {} ms (Raspi->BMS 0xB000/B101–B106/B111–B112)",
            BMS_TELEMETRY_POLL_INTERVAL.as_millis()
        );

        // DAQ Nucleo polling (IMU/temp) is off by default - it floods can0 in BMS lab.
        // Re-enable with: CAN_DAQ_POLL=1
        if std::env::var_os("CAN_DAQ_POLL").is_some_and(|v| v == "1") {
            spawn_can_polling_tasks(socket, event_tx.clone());
            println!("DAQ CAN polling enabled (CAN_DAQ_POLL=1)");
        }
    }

    tokio::spawn(device_timeout_task(
        Arc::clone(&registry),
        event_tx.clone(),
    ));

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("client connected: {addr}");
        let registry = Arc::clone(&registry);
        let event_tx = event_tx.clone();
        let can_socket = can_socket.clone();

        tokio::spawn(async move {
            if let Err(error) = handle_client(stream, addr, registry, event_tx, can_socket).await {
                eprintln!("client {addr} error: {error}");
            }
        });
    }
}

fn open_can_socket() -> Option<CanSocket> {
    let interface = std::env::var("CAN_INTERFACE").unwrap_or_else(|_| "can0".to_string());

    match CanSocket::open(interface.as_str()) {
        Ok(socket) => {
            println!("CAN socket opened on {interface}");
            Some(socket)
        }
        Err(error) => {
            eprintln!("failed to open CAN socket on {interface}: {error}");
            None
        }
    }
}

async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    registry: Arc<Mutex<DeviceRegistry>>,
    event_tx: broadcast::Sender<BackendEvent>,
    can_socket: Option<CanSocket>,
) -> Result<(), Box<dyn Error>> {
    let ws = accept_async(stream).await?;
    let (mut ws_tx, mut ws_rx) = ws.split();
    let mut event_rx = event_tx.subscribe();

    let snapshot = {
        let registry = registry.lock().await;
        bridge::device_status_snapshot(&registry, Instant::now())
    };
    ws_tx.send(encode_outgoing(&snapshot)?).await?;

    loop {
        tokio::select! {
            message = ws_rx.next() => {
                let Some(message) = message else {
                    break;
                };

                match message {
                    Ok(frame) => {
                        if frame.is_close() {
                            break;
                        }

                        match decode_incoming(&frame) {
                            Ok(Some(command)) => {
                                if let Err(message) = handle_frontend_command(
                                    &command.data,
                                    can_socket.as_ref(),
                                    &registry,
                                    &event_tx,
                                )
                                .await
                                {
                                    let event = backend_event(BackendEventData::BackendError {
                                        message: format!("charging command failed: {message}"),
                                    });
                                    ws_tx.send(encode_outgoing(&event)?).await?;
                                }
                            }
                            Ok(None) => {}
                            Err(error) => {
                                let event = backend_event(BackendEventData::BackendError {
                                    message: format!("invalid frontend command: {error}"),
                                });
                                ws_tx.send(encode_outgoing(&event)?).await?;
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("client {addr} rx error: {error}");
                        break;
                    }
                }
            }
            event = event_rx.recv() => {
                match event {
                    Ok(event) => ws_tx.send(encode_outgoing(&event)?).await?,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let event = backend_event(BackendEventData::BackendError {
                            message: format!("websocket client skipped {skipped} backend events"),
                        });
                        ws_tx.send(encode_outgoing(&event)?).await?;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };
    }

    println!("client disconnected: {addr}");
    Ok(())
}

async fn handle_frontend_command(
    command: &websocket::FrontendCommandData,
    can_socket: Option<&CanSocket>,
    registry: &Mutex<DeviceRegistry>,
    event_tx: &broadcast::Sender<BackendEvent>,
) -> Result<(), String> {
    let Some(socket) = can_socket else {
        return Err("CAN socket is not available".to_string());
    };

    {
        let registry = registry.lock().await;
        let bms_online = registry.get(CanNode::Bms).is_some_and(|d| d.online);
        if !bms_online {
            return Err("BMS is offline — refusing charging CAN TX".to_string());
        }
    }

    let frames = bridge::can_messages_for_frontend_command(command).map_err(|e| e.to_string())?;
    for frame in &frames {
        if let Err(error) = socket.write_message(frame).await {
            let event = backend_event(BackendEventData::BackendError {
                message: format!("CAN TX error: {error}"),
            });
            let _ = event_tx.send(event);
            return Err(error.to_string());
        }
        println!(
            "CAN TX cmd=0x{:04X} dlc={} data={}",
            u16::from(frame.id.command),
            frame.data.len(),
            frame
                .data
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    Ok(())
}

async fn device_timeout_task(
    registry: Arc<Mutex<DeviceRegistry>>,
    event_tx: broadcast::Sender<BackendEvent>,
) {
    let mut interval = time::interval(DEVICE_TIMEOUT_CHECK_INTERVAL);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        let now = Instant::now();
        let mut went_offline = Vec::new();

        {
            let mut registry = registry.lock().await;
            let previously_online: Vec<CanNode> = registry.online().map(|d| d.node).collect();
            registry.mark_timeouts(now, DEVICE_OFFLINE_TIMEOUT);
            for node in previously_online {
                if registry.get(node).is_some_and(|d| !d.online) {
                    went_offline.push(node);
                }
            }

            for node in went_offline {
                if let Some(event) = bridge::device_status_changed(&registry, node, now) {
                    let _ = event_tx.send(event);
                }
            }
        }
    }
}

async fn can_receive_task(
    socket: CanSocket,
    registry: Arc<Mutex<DeviceRegistry>>,
    event_tx: broadcast::Sender<BackendEvent>,
) {
    let mut last_status_broadcast = HashMap::<CanNode, Instant>::new();

    loop {
        match socket.read_message().await {
            Ok(Some(message)) => {
                let now = Instant::now();
                let should_broadcast_status = last_status_broadcast
                    .get(&message.id.source)
                    .map(|last_broadcast| {
                        now.duration_since(*last_broadcast) >= DEVICE_STATUS_BROADCAST_INTERVAL
                    })
                    .unwrap_or(true);

                let status_event = if should_broadcast_status {
                    let mut registry = registry.lock().await;
                    registry.mark_seen(message.id.source, now);
                    bridge::device_status_changed(&registry, message.id.source, now)
                } else {
                    let mut registry = registry.lock().await;
                    registry.mark_seen(message.id.source, now);
                    None
                };

                if let Some(event) = status_event {
                    last_status_broadcast.insert(message.id.source, now);
                    let _ = event_tx.send(event);
                }

                match bridge::telemetry_events_for_can_message(&message) {
                    Ok(events) => {
                        for event in events {
                            let _ = event_tx.send(event);
                        }
                    }
                    Err(error) => {
                        let event = backend_event(BackendEventData::BackendError {
                            message: format!("failed to decode CAN telemetry: {error}"),
                        });
                        let _ = event_tx.send(event);
                    }
                }
            }
            Ok(None) => {}
            Err(error) => {
                let event = backend_event(BackendEventData::BackendError {
                    message: format!("CAN socket read error: {error}"),
                });
                let _ = event_tx.send(event);
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CanPollingSchedule {
    target: CanNode,
    request_command: DaqCanCommand,
    interval: Duration,
}

const CAN_POLLING_SCHEDULES: &[CanPollingSchedule] = &[
    CanPollingSchedule {
        target: CanNode::Nucleo1,
        request_command: DaqCanCommand::ReqImuData,
        interval: Duration::from_millis(20),
    },
    CanPollingSchedule {
        target: CanNode::Nucleo1,
        request_command: DaqCanCommand::ReqTempData,
        interval: Duration::from_millis(100),
    },
];

fn spawn_bms_polling_task(socket: CanSocket, event_tx: broadcast::Sender<BackendEvent>) {
    tokio::spawn(async move {
        bms_polling_task(socket, event_tx).await;
    });
}

async fn bms_polling_task(socket: CanSocket, event_tx: broadcast::Sender<BackendEvent>) {
    let mut interval = time::interval(BMS_TELEMETRY_POLL_INTERVAL);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        for message in bridge::bms_telemetry_poll_messages() {
            if let Err(error) = socket.write_message(&message).await {
                let cmd: u16 = match message.id.command {
                    CanCommand::Bms(command) => command.into(),
                    _ => 0,
                };
                let event = backend_event(BackendEventData::BackendError {
                    message: format!(
                        "failed to send BMS telemetry poll 0x{cmd:04X} to {:?}: {error}",
                        CanNode::Bms,
                    ),
                });
                let _ = event_tx.send(event);
                break;
            }
            // Let the BMS (or emulator) answer between requests.
            tokio::task::yield_now().await;
        }
    }
}

fn spawn_can_polling_tasks(socket: CanSocket, event_tx: broadcast::Sender<BackendEvent>) {
    for schedule in CAN_POLLING_SCHEDULES {
        let socket = socket.clone();
        let event_tx = event_tx.clone();
        let schedule = *schedule;

        tokio::spawn(async move {
            can_polling_task(socket, schedule, event_tx).await;
        });
    }
}

async fn can_polling_task(
    socket: CanSocket,
    schedule: CanPollingSchedule,
    event_tx: broadcast::Sender<BackendEvent>,
) {
    let mut interval = time::interval(schedule.interval);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let message = DfrCanMessageBuf {
            id: DfrCanId {
                priority: 1,
                target: schedule.target,
                source: CanNode::Raspi,
                command: CanCommand::Daq(schedule.request_command),
            },
            data: Vec::new(),
        };

        if let Err(error) = socket.write_message(&message).await {
            let event = backend_event(BackendEventData::BackendError {
                message: format!(
                    "failed to send CAN polling request {:?} to {:?}: {error}",
                    schedule.request_command, schedule.target,
                ),
            });
            let _ = event_tx.send(event);
        }
    }
}
