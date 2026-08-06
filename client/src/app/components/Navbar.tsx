import { Battery, Power, Moon, Sun } from 'lucide-react';
import { useTheme } from 'next-themes';
import { useState } from 'react';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from './ui/alert-dialog';

import { invoke } from '@tauri-apps/api/core';

interface NavbarProps {
  isConnected?: boolean;
}

export function Navbar({ isConnected = false }: NavbarProps) {
  const { theme, setTheme } = useTheme();
  const [showPowerDialog, setShowPowerDialog] = useState(false);

  const handlePowerOff = async () => {
    try {
      await invoke('exit_app');
    } catch (err) {
      console.error('Failed to exit app:', err);
    } finally {
      setShowPowerDialog(false);
    }
  };


  return (
    <>
      <nav className="h-16 bg-background border-b border-border flex items-center justify-between px-6 backdrop-blur-xl bg-opacity-80">
        <div className="flex items-center gap-3">
          <Battery className="w-6 h-6 text-primary" />
          <h1 className="text-lg tracking-wide font-medium">EV BATTERY MANAGEMENT SYSTEM</h1>
        </div>
        <div className="flex items-center gap-6">
          <div
            className="flex items-center gap-2"
            title={isConnected ? 'BMS node online' : 'BMS node offline — see Status'}
          >
            <div
              className={`w-2 h-2 rounded-full ${
                isConnected ? 'bg-green-500 animate-pulse' : 'bg-muted-foreground/50'
              }`}
            />
            <span
              className={`text-sm uppercase tracking-wider ${
                isConnected ? 'text-green-500' : 'text-muted-foreground'
              }`}
            >
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
          <button
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            className="p-2 hover:bg-accent rounded-lg transition-colors"
          >
            {theme === 'dark' ? (
              <Sun className="w-5 h-5 text-muted-foreground" />
            ) : (
              <Moon className="w-5 h-5 text-muted-foreground" />
            )}
          </button>
          <button
            onClick={() => setShowPowerDialog(true)}
            className="p-2 hover:bg-accent rounded-lg transition-colors"
            title="Power Off"
          >
            <Power className="w-5 h-5 text-muted-foreground hover:text-red-500 transition-colors" />
          </button>
        </div>
      </nav>

      <AlertDialog open={showPowerDialog} onOpenChange={setShowPowerDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Disconnect and Quit?</AlertDialogTitle>
            <AlertDialogDescription>
              This will disconnect from the BMS and close the application.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handlePowerOff}>Confirm</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}