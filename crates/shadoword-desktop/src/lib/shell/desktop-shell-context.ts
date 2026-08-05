import { createContext } from 'svelte';
import type { DesktopShellState } from './desktop-shell.svelte';

export const [useDesktopShell, provideDesktopShell] = createContext<DesktopShellState>();
