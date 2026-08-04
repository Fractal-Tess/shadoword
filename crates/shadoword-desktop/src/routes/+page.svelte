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
  everywhere except the one riveted identity plate.
STORY: The operator confirms which machine will do the work, holds one
  unmistakable control, watches the void column close or stay open, and takes the
  text.
FIRST VIEWPORT: Command rail left, capture stage centre carrying the spectrum
  raster and the void column, signal-path rail right. The low-frequency rail runs
  along the window's bottom edge under everything.
FORM: Neo-Tokyo Neon Night, abstracted from cityscape to spectrum; seed 322d3899.
-->
<script lang="ts">
	import { browser } from '$app/environment';
	import { DesktopAppState } from '$lib/app-state.svelte';
	import CommandRail from '$lib/components/CommandRail.svelte';
	import ContextStrip from '$lib/components/ContextStrip.svelte';
	import AboutView from '$lib/views/AboutView.svelte';
	import HistoryView from '$lib/views/HistoryView.svelte';
	import ModelsView from '$lib/views/ModelsView.svelte';
	import SettingsView from '$lib/views/SettingsView.svelte';
	import TranscribeView from '$lib/views/TranscribeView.svelte';
	import WindowChrome from '$lib/components/WindowChrome.svelte';
	import type { PageId } from '$lib/types';
	import { inferencePoolSummary } from '$lib/inference-pool';
	import { onMount } from 'svelte';

	const requestedPage = browser ? new URLSearchParams(window.location.search).get('page') : null;
	const initialPage =
		requestedPage === 'models' ||
		requestedPage === 'history' ||
		requestedPage === 'settings' ||
		requestedPage === 'about'
			? requestedPage
			: 'transcribe';

	let activePage = $state<PageId>(initialPage);
	const app = new DesktopAppState(
		browser && new URLSearchParams(window.location.search).get('demo') === '1'
	);
	let mode = $derived(app.settings?.mode ?? 'remote');
	let poolSummary = $derived(inferencePoolSummary(app.overview?.status.inference_pool));

	onMount(() => {
		void app.initialize();
		return () => app.dispose();
	});
</script>

<svelte:head>
	<title>Shadoword · Private speech to text</title>
	<meta name="description" content="Shadoword private speech-to-text desktop application." />
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
	<WindowChrome {activePage} />
	<CommandRail {app} bind:activePage />
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
			<!-- Below 1180px the right rail is gone, so the signal path collapses into
			     one line. Same three stages, same order, same marking on the middle one. -->
			<div class="compact-signal-summary mono-micro" aria-label="Current signal path">
				<span>{app.settings?.input_device ?? 'System default'}</span>
				<i aria-hidden="true"></i>
				<!-- Only the machine name takes the accent. The pool counts rode it too in
				     the first pass, which put four scarlet words in a row and made the one
				     thing that matters — which machine is doing the work — impossible to pick
				     out of its own highlight. -->
				<strong>{mode === 'remote' ? 'Remote API' : 'This machine'}</strong>
				<em>{poolSummary}</em>
				<i aria-hidden="true"></i>
				<span
					>{app.settings?.paste_method === 'direct'
						? 'Type directly'
						: app.settings?.copy_to_clipboard
							? 'Clipboard'
							: 'Transcript surface'}</span
				>
			</div>
			<div class="sr-only" aria-live="polite">
				{activePage} view · {mode === 'remote' ? 'Remote API' : 'Local machine'} · {poolSummary}
			</div>
			{#if activePage === 'transcribe'}
				<TranscribeView {app} />
			{:else if activePage === 'models'}
				<ModelsView {app} />
			{:else if activePage === 'history'}
				<HistoryView {app} />
			{:else if activePage === 'settings'}
				{#key app.settings}
					<SettingsView {app} />
				{/key}
			{:else}
				<AboutView />
			{/if}
		</div>
	</main>
	<ContextStrip {app} />
</div>

<style>
	.app-shell {
		display: grid;
		grid-template-columns: 14rem minmax(0, 1fr) 14rem;
		grid-template-rows: 2.75rem minmax(0, 1fr);
		min-width: 0;
		height: 100dvh;
		overflow: hidden;
		background: var(--surface-0);
	}

	main {
		grid-column: 2;
		grid-row: 2;
		min-width: 0;
		overflow: auto;
		background: var(--surface-0);
		scrollbar-color: var(--line-strong) transparent;
	}

	.work-surface {
		width: min(100%, 72rem);
		min-height: 100%;
		margin: 0 auto;
		padding: clamp(1.4rem, 2.6vw, 2.25rem) clamp(1.4rem, 2.6vw, 2.25rem) 3.5rem;
	}

	.compact-signal-summary {
		display: none;
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
		grid-row: 2;
	}

	:global(.context-strip) {
		grid-column: 3;
		grid-row: 2;
	}

	@media (max-width: 1180px) {
		.app-shell {
			grid-template-columns: 14rem minmax(0, 1fr);
		}

		.compact-signal-summary {
			display: flex;
			align-items: center;
			gap: 0.6rem;
			margin: 0 0 1.15rem;
			border: 1px solid var(--line);
			padding: 0.5rem 0.7rem;
			background: var(--surface-1);
			color: var(--ink-muted);
		}

		.compact-signal-summary strong {
			flex-shrink: 0;
			color: var(--scarlet-lamp);
			font-weight: 400;
			white-space: nowrap;
		}

		.compact-signal-summary em {
			min-width: 0;
			overflow: hidden;
			color: var(--ink-dim);
			font-style: normal;
			text-overflow: ellipsis;
			white-space: nowrap;
		}

		/* The last stage is the first thing to go when the line runs out of room: the
		   delivery target is also printed in the transcript surface's own footer, and
		   the input device is not. */
		.compact-signal-summary > span:last-of-type {
			min-width: 0;
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}

		/* The joining line from the full rail, reduced to a 0.7rem segment between
		   stages. Still one path, still drawn rather than punctuated with arrows. */
		.compact-signal-summary i {
			width: 0.7rem;
			height: 1px;
			flex-shrink: 0;
			background: var(--line-strong);
		}
	}

	@media (max-width: 900px) {
		.app-shell {
			grid-template-columns: 4.5rem minmax(0, 1fr);
		}

		.work-surface {
			padding: 1.35rem 1.35rem 7rem;
		}
	}
</style>
