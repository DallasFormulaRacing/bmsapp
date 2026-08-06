import { Navbar } from './components/Navbar';
import { Sidebar } from './components/Sidebar';
import { BatterySegment } from './components/BatterySegment';
import { DraggableOverviewStats } from './components/DraggableOverviewStats';
import { SegmentDetailedStats } from './components/SegmentDetailedStats';
import { StatusPage } from './components/StatusPage';
import { ChargingDashboard } from './components/ChargingDashboard';
import { AnalyticsPage } from './components/AnalyticsPage';
import { InfoPage } from './components/InfoPage';
import { ThemeProvider } from 'next-themes';
import { useState, useCallback, useMemo } from 'react';
import { DndProvider } from 'react-dnd';
import { HTML5Backend } from 'react-dnd-html5-backend';
import { Flame } from 'lucide-react';
import { useBridgeSocket } from '../hooks/useBridgeSocket';
import { useAnalyticsHistory } from '../hooks/useAnalyticsHistory';
import { deriveSegmentStats } from '../lib/battery';

export default function App() {
  const [activeTab, setActiveTab] = useState('overview');
  const [cardOrder, setCardOrder] = useState([0, 1, 2, 3, 4, 5, 6, 7, 8]);
  const [heatmapEnabled, setHeatmapEnabled] = useState(false);

  const bridge = useBridgeSocket();
  const { pack, segments, linkStatus, reconnect, startCharging, stopCharging } = bridge;
  const bmsConnected = linkStatus.bms.health === 'up';
  const { data: analyticsData, peaks: analyticsPeaks } = useAnalyticsHistory(pack);

  const segmentStats = useMemo(() => deriveSegmentStats(segments), [segments]);

  const cellDelta =
    pack.minCellVoltage != null && pack.maxCellVoltage != null
      ? pack.maxCellVoltage - pack.minCellVoltage
      : null;

  const packPower =
    pack.packVoltage != null && pack.packCurrent != null
      ? pack.packVoltage * pack.packCurrent
      : null;

  const handleReorder = useCallback((dragIndex: number, hoverIndex: number) => {
    setCardOrder((prevOrder) => {
      const newOrder = [...prevOrder];
      const [removed] = newOrder.splice(dragIndex, 1);
      newOrder.splice(hoverIndex, 0, removed);
      return newOrder;
    });
  }, []);

  return (
    <ThemeProvider attribute="class" defaultTheme="dark" enableSystem={false}>
      <DndProvider backend={HTML5Backend}>
        <div className="h-screen flex flex-col bg-background relative overflow-hidden">
          <div className="absolute top-0 right-0 w-96 h-96 bg-primary/5 rounded-full blur-[120px] pointer-events-none" />
          <div className="absolute bottom-0 left-0 w-96 h-96 bg-primary/5 rounded-full blur-[120px] pointer-events-none" />

          <Navbar isConnected={bmsConnected} />
          <div className="flex flex-1 overflow-hidden relative z-10">
            <Sidebar activeTab={activeTab} onTabChange={setActiveTab} />
            <main className="flex-1 overflow-auto p-6">
              {activeTab === 'overview' && (
                <>
                  <div className="grid grid-cols-[minmax(0,55fr)_minmax(0,45fr)] gap-4 mb-6 items-stretch">
                    <div className="min-w-0">
                      <h2 className="mb-4 text-base uppercase tracking-wider text-primary/80">
                        Battery Overview
                      </h2>
                      <DraggableOverviewStats
                        totalVoltage={pack.packVoltage}
                        avgTemperature={pack.avgTemp}
                        stateOfCharge={pack.stateOfCharge}
                        cellDelta={cellDelta}
                        powerStatus={pack.powerStatus}
                        balancingActive={pack.balancingActive}
                        totalCurrent={pack.packCurrent}
                        packPower={packPower}
                        minVoltage={pack.minCellVoltage}
                        maxVoltage={pack.maxCellVoltage}
                        cardOrder={cardOrder}
                        onReorder={handleReorder}
                      />
                    </div>
                    <div className="min-w-0 flex flex-col">
                      <h2 className="mb-4 text-base uppercase tracking-wider text-primary/80">
                        Charging
                      </h2>
                      <div className="flex-1 min-h-0">
                        <ChargingDashboard
                          bmsOnline={bmsConnected}
                          onStartCharging={startCharging}
                          onStopCharging={stopCharging}
                        />
                      </div>
                    </div>
                  </div>

                  <SegmentDetailedStats segments={segmentStats} />
                </>
              )}

              {activeTab === 'segments' && (
                <>
                  <div className="flex items-center justify-between mb-4">
                    <h2 className="text-base uppercase tracking-wider text-primary/80">
                      Battery Segments
                    </h2>
                    <button
                      onClick={() => setHeatmapEnabled(!heatmapEnabled)}
                      className={`flex items-center gap-1.5 px-3 py-1.5 rounded text-sm hover:bg-muted transition-all duration-300 uppercase tracking-wide ${
                        heatmapEnabled ? 'bg-orange-500/20 text-orange-500' : 'text-muted-foreground'
                      }`}
                      title="Toggle Heatmap"
                    >
                      <Flame className="w-4 h-4" />
                      <span>Heatmap</span>
                    </button>
                  </div>
                  <div className="grid grid-cols-3 gap-3">
                    {segments.map((segment) => (
                      <BatterySegment
                        key={segment.segmentNumber}
                        segmentNumber={segment.segmentNumber}
                        cells={segment.cells}
                        minVoltageGlobal={pack.minCellVoltage}
                        maxVoltageGlobal={pack.maxCellVoltage}
                        maxTempGlobal={pack.maxTemp}
                        heatmapEnabled={heatmapEnabled}
                      />
                    ))}
                  </div>
                </>
              )}

              {activeTab === 'info' && <InfoPage />}

              {activeTab === 'status' && (
                <StatusPage status={linkStatus} onReconnect={reconnect} />
              )}

              {activeTab === 'analytics' && (
                <AnalyticsPage data={analyticsData} peaks={analyticsPeaks} />
              )}
            </main>
          </div>
        </div>
      </DndProvider>
    </ThemeProvider>
  );
}
