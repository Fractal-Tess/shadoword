import type { DesktopAppState } from '$lib/app-state.svelte';
import { commands } from '$lib/bindings';

export class MicrophoneMonitorState {
	readonly app: DesktopAppState;
	level = $state(0);
	monitoring = $state(false);
	error = $state('');

	constructor(app: DesktopAppState) {
		this.app = app;
	}

	get percent() {
		return Math.round(this.level * 100);
	}

	start() {
		let disposed = false;
		let demoPhase = 0;
		const poll = async () => {
			if (this.app.demo) {
				demoPhase += 0.45;
				this.level = 0.12 + Math.abs(Math.sin(demoPhase)) * 0.52;
				this.monitoring = true;
				return;
			}
			try {
				const level = await commands.pollMicrophoneLevel();
				if (disposed) return;
				this.monitoring = level.monitoring;
				this.error = '';
				this.level = Math.max(level.peak ?? 0, this.level * 0.68);
			} catch {
				if (disposed) return;
				this.monitoring = false;
				this.error = 'Microphone level unavailable';
				this.level = 0;
			}
		};

		void poll();
		const interval = window.setInterval(() => void poll(), 100);
		return () => {
			disposed = true;
			window.clearInterval(interval);
			if (!this.app.demo) void commands.stopMicrophoneLevelMonitor();
		};
	}
}
