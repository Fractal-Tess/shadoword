<script lang="ts">
	import {
		AppWindow,
		AudioLines,
		Box,
		Captions,
		Clock3,
		Info,
		Mic2,
		Send,
		Settings2
	} from '@lucide/svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import type { PageId } from '$lib/types';

	const shell = useDesktopShell();
	const destinations = [
		{ id: 'transcribe', label: 'Transcribe', icon: Mic2 },
		{ id: 'models', label: 'Models', icon: Box },
		{ id: 'history', label: 'History', icon: Clock3 },
		{ id: 'settings', label: 'Execution', icon: Settings2 },
		{ id: 'capture', label: 'Capture', icon: AudioLines },
		{ id: 'transcription', label: 'Transcription', icon: Captions },
		{ id: 'output', label: 'Output', icon: Send },
		{ id: 'application', label: 'Application', icon: AppWindow },
		{ id: 'about', label: 'About', icon: Info }
	] satisfies Array<{ id: PageId; label: string; icon: typeof Mic2 }>;
</script>

<nav aria-label="Primary navigation">
	{#each destinations.filter( (destination) => shell.isPageAvailable(destination.id) ) as destination (destination.id)}
		{@const Icon = destination.icon}
		<button
			type="button"
			class:active={shell.activePage === destination.id}
			onclick={() => shell.navigate(destination.id)}
			aria-current={shell.activePage === destination.id ? 'page' : undefined}
			aria-label={destination.label}
		>
			{#if shell.activePage === destination.id}
				<span class="marker" aria-hidden="true"></span>
			{/if}
			<Icon size={20} strokeWidth={1.8} aria-hidden="true" />
			<span class="destination-label display-legend">{destination.label}</span>
		</button>
	{/each}
</nav>

<style>
	nav {
		display: grid;
		gap: 1px;
		padding: 0.5rem 0;
	}

	button {
		position: relative;
		display: grid;
		grid-template-columns: 1.25rem 1fr;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		min-height: 2.35rem;
		border: 0;
		padding: 0 1.15rem;
		background: transparent;
		color: var(--ink-muted);
		text-align: left;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	button:hover,
	button.active {
		background: var(--surface-2);
		color: var(--ink);
	}

	button:focus-visible {
		outline: 2px solid var(--ink);
		outline-offset: -2px;
	}

	.destination-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.marker {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 0;
		width: 2px;
		background: var(--scarlet);
	}

	@media (max-width: 900px) {
		button {
			grid-template-columns: 1.25rem;
			justify-content: center;
			padding: 0;
		}

		.destination-label {
			display: none;
		}
	}
</style>
