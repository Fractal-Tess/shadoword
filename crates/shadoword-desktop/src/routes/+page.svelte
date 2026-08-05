<!--
IMPECCABLE DIRECTION CONTRACT
THESIS: An instrument sitting on a blazing spectrum with one silent channel in it.
  The same world the public site ships, translated from Persuade to Operate: the
  imagery recedes to the window's edges and the argument is carried by material,
  not by a hero.
OWN-WORLD: Rain-black grounds. Scarlet is the only accent, split into pigment
  (fills, hairlines) and lamp (type), and it is rationed by meaning — the selected
  destination, the marked inference stage, the record slab. Magenta and cyan exist
  only inside the raster. Heavy condensed grotesque against a monospace. Radius 0
  everywhere except the one riveted identity plate. Magenta and cyan exist only
  inside the generated spectrum field.
STORY: The operator confirms which machine will do the work, holds one
  unmistakable control, watches the void column close or stay open, and takes the
  text.
FIRST VIEWPORT: Command rail left, capture stage and transcript surface filling
  the remaining window. The low-frequency rail stays inside the capture stage.
FORM: Neo-Tokyo Neon Night, abstracted from cityscape to spectrum; seed 322d3899.
-->
<script lang="ts">
	import { browser } from '$app/environment';
	import { DesktopAppState } from '$lib/app-state.svelte';
	import CommandRail from '$lib/components/CommandRail.svelte';
	import AboutView from '$lib/views/AboutView.svelte';
	import HistoryView from '$lib/views/HistoryView.svelte';
	import ModelsView from '$lib/views/ModelsView.svelte';
	import SettingsView from '$lib/views/SettingsView.svelte';
	import TranscribeView from '$lib/views/TranscribeView.svelte';
	import { inferencePoolSummary } from '$lib/inference-pool';
	import { DesktopShellState } from '$lib/shell/desktop-shell.svelte';
	import { provideDesktopShell } from '$lib/shell/desktop-shell-context';
	import { onMount } from 'svelte';

	const requestedPage = browser ? new URLSearchParams(window.location.search).get('page') : null;
	const initialPage =
		requestedPage === 'models' ||
		requestedPage === 'history' ||
		requestedPage === 'settings' ||
		requestedPage === 'capture' ||
		requestedPage === 'transcription' ||
		requestedPage === 'output' ||
		requestedPage === 'application' ||
		requestedPage === 'about'
			? requestedPage
			: 'transcribe';

	const app = new DesktopAppState(
		browser && new URLSearchParams(window.location.search).get('demo') === '1'
	);
	const shell = new DesktopShellState(app, initialPage);
	provideDesktopShell(shell);
	let mode = $derived(shell.mode);
	let poolSummary = $derived(inferencePoolSummary(app.overview?.status.inference_pool));

	onMount(() => {
		void app.initialize().then(() => shell.reconcilePage());
		return () => app.dispose();
	});
</script>

<svelte:head>
	<title>Shadoword · Speech to text, local by default</title>
	<meta
		name="description"
		content="Shadoword speech to text with local, self-hosted, and OpenRouter execution."
	/>
</svelte:head>

<div class="app-shell">
	<!--
		No imagery at this level. There was a spectrum band pinned to the window's
		bottom edge here, and it had to go: the capture stage already carries the
		spectrum, and it carries it registered to the local/remote choice, which is the
		only place the imagery means anything. A second band underneath every view
		turned the world into wallpaper — History and Settings were reading over lit
		bins for no reason, and the mark that matters stopped being the only one. In
		Operate mode the shell's job is to be a window, not a picture.
	-->
	<CommandRail />
	<main tabindex="-1">
		<div class="work-surface">
			{#if app.demo}
				<div class="demo-banner mono-micro" role="status">
					Development demo · fixture data and simulated actions
				</div>
			{/if}
			{#if app.error}
				<div class="global-error mono-micro" role="alert">
					<span>{app.error}</span>
					<button type="button" onclick={() => app.retryError()}>
						{app.errorRetry ? 'Retry' : 'Dismiss'}
					</button>
				</div>
			{/if}
			<div class="sr-only" aria-live="polite">
				{shell.activePage} view · {mode == null
					? 'Loading execution target'
					: mode === 'remote'
						? 'Shadoword API'
						: mode === 'open_router'
							? 'OpenRouter'
							: 'Local machine'} · {poolSummary}
			</div>
			{#if shell.activePage === 'transcribe'}
				<TranscribeView />
			{:else if shell.activePage === 'models'}
				<ModelsView />
			{:else if shell.activePage === 'history'}
				<HistoryView />
			{:else if shell.activePage === 'settings' || shell.activePage === 'capture' || shell.activePage === 'transcription' || shell.activePage === 'output' || shell.activePage === 'application'}
				{#if app.settings}
					<SettingsView section={shell.activePage} />
				{:else}
					<div class="settings-loading mono-caption" role="status">Loading native settings…</div>
				{/if}
			{:else}
				<AboutView />
			{/if}
		</div>
	</main>
</div>

<style>
	.app-shell {
		display: grid;
		grid-template-columns: 14rem minmax(0, 1fr);
		min-width: 0;
		width: 100%;
		height: 100svh;
		overflow: hidden;
		background: var(--surface-0);
	}

	main {
		grid-column: 2;
		min-width: 0;
		overflow: auto;
		background: var(--surface-0);
		scrollbar-color: var(--line-strong) transparent;
	}

	.work-surface {
		display: flex;
		width: min(100%, 72rem);
		height: 100%;
		min-height: 0;
		margin: 0 auto;
		padding: clamp(1.1rem, 2vw, 1.75rem);
		flex-direction: column;
	}

	.work-surface :global(.transcribe-view) {
		min-height: 0;
		flex: 1;
	}

	.settings-loading {
		border: 1px solid var(--line);
		padding: 1rem;
		color: var(--ink-muted);
	}

	.demo-banner,
	.global-error {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1rem;
		border: 1px solid var(--line-strong);
		padding: 0.6rem 0.8rem;
		background: var(--surface-1);
		color: var(--ink-dim);
	}

	/* The error notice is the one thing in this window allowed a scarlet edge on
	   both sides: a rule is not type, and an alert that has to be found is exactly
	   what the accent is rationed for. The message itself stays off-white — scarlet
	   prose at 12px is a legibility problem dressed as urgency. */
	.global-error {
		border-color: var(--scarlet);
		border-left-width: 2px;
	}

	.global-error button {
		flex-shrink: 0;
		border: 1px solid var(--scarlet);
		padding: 0.3rem 0.7rem;
		background: transparent;
		color: var(--scarlet-lamp);
		font: inherit;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	.global-error button:hover {
		background: var(--scarlet);
		color: var(--on-scarlet);
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}

	:global(.command-rail) {
		grid-column: 1;
	}

	@media (max-width: 900px) {
		.app-shell {
			grid-template-columns: 4.5rem minmax(0, 1fr);
		}

		.work-surface {
			padding: 1.1rem;
		}
	}
</style>
