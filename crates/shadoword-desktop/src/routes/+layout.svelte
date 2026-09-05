<!--
IMPECCABLE DIRECTION CONTRACT
THESIS: An instrument sitting on a blazing spectrum with one silent channel in it.
OWN-WORLD: Rain-black grounds. Scarlet is the only interface accent. Heavy condensed
  grotesque against a monospace. Radius 0 everywhere except the one riveted identity plate.
STORY: The operator confirms which machine will do the work, holds one unmistakable
  control, watches the void column close or stay open, and takes the text.
FORM: Neo-Tokyo Neon Night, abstracted from cityscape to spectrum; seed 322d3899.
-->
<script lang="ts">
	import { afterNavigate, beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { DesktopAppState } from '$lib/app-state.svelte';
	import favicon from '$lib/assets/favicon.svg';
	import BlinkingSquares from './_shell/BlinkingSquares.svelte';
	import { BrandMark } from '$lib/components/ui/brand-mark';
	import { Button } from '$lib/components/ui/button';
	import { Toaster } from '$lib/components/ui/sonner';
	import EnvironmentSelector from './_shell/EnvironmentSelector.svelte';
	import PrimaryNavigation from './_shell/PrimaryNavigation.svelte';
	import RuntimeStation from './_shell/RuntimeStation.svelte';
	import WindowTitleBar from './_shell/WindowTitleBar.svelte';
	import { inferencePoolSummary } from '$lib/inference-pool';
	import { DesktopShellState } from '$lib/shell/desktop-shell.svelte';
	import { provideDesktopShell } from '$lib/shell/desktop-shell-context';
	import { isSettingsPage, pageIdFromPathname } from '$lib/shell/routes';
	import { onMount } from 'svelte';
	import './layout.css';

	let { children } = $props();
	const app = new DesktopAppState(page.url.searchParams.get('demo') === '1');
	const shell = new DesktopShellState(app);
	provideDesktopShell(shell);

	let mode = $derived(shell.mode);
	let poolSummary = $derived(inferencePoolSummary(app.overview?.status.inference_pool));

	beforeNavigate((navigation) => {
		const target = navigation.to?.url;
		const staysWithinSettings =
			target &&
			isSettingsPage(pageIdFromPathname(page.url.pathname)) &&
			isSettingsPage(pageIdFromPathname(target.pathname));
		if (
			!target ||
			navigation.willUnload ||
			staysWithinSettings ||
			!shell.shouldGuardNavigation(target)
		)
			return;
		navigation.cancel();
		void shell.continueGuardedNavigation(
			target,
			navigation.type === 'popstate' ? navigation.delta : null
		);
	});

	afterNavigate((navigation) => {
		void shell.canonicalizeLegacyRoute();
		if (!navigation.from) return;
		void shell.focusWorkSurface();
		void shell.reconcilePage();
	});

	onMount(() => {
		const uninstallWindowCloseHandler = shell.installWindowCloseHandler();
		void app.initialize().then(() => shell.reconcilePage());
		return () => {
			uninstallWindowCloseHandler();
			app.dispose();
		};
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta
		name="description"
		content="Shadoword speech to text with local, self-hosted, and OpenRouter execution."
	/>
</svelte:head>

<div class="flex h-svh w-full min-w-0 flex-col overflow-hidden bg-night">
	{#if app.settings?.show_window_title_bar !== false}
		<WindowTitleBar />
	{/if}
	<div
		class="relative isolate grid min-h-0 flex-1 grid-cols-[14rem_minmax(0,1fr)] overflow-hidden bg-night max-[999px]:grid-cols-[4.5rem_minmax(0,1fr)]"
	>
		<BlinkingSquares
			class="pointer-events-none absolute inset-0 z-0"
			fill
			active={app.recording}
			direction="right"
			gridSize={74}
			falloff={1.35}
			fadeStart={0.12}
			squareSize={0.22}
			minBrightness={0.4}
			twinkleSpeed={2}
			twinkleStrength={0.65}
			intensity={0.9}
			opacity={0.78}
			dpr={1.25}
		/>
		<aside
			class="relative z-[1] col-start-1 flex min-h-0 min-w-0 [scrollbar-color:var(--line-strong)_transparent] flex-col overflow-y-auto overscroll-contain border-r border-line bg-plate"
			aria-label="Shadoword controls"
		>
			<div
				class="grid shrink-0 place-items-center border-b border-line px-[1.15rem] py-[0.9rem] max-[999px]:px-0 max-[999px]:[&_[data-brand-wordmark]]:hidden"
			>
				<BrandMark />
			</div>
			<EnvironmentSelector />
			<PrimaryNavigation />
			<RuntimeStation />
		</aside>
		<main
			{@attach shell.workSurfaceAttachment}
			tabindex="-1"
			class="relative z-[1] col-start-2 min-w-0 [scrollbar-color:var(--line-strong)_transparent] overflow-auto bg-transparent"
		>
			<div
				class="mx-auto flex h-full min-h-0 w-[min(100%,72rem)] flex-col p-[clamp(1.1rem,2vw,1.75rem)] max-[999px]:p-[1.1rem]"
			>
				{#if app.demo}
					<div
						class="mb-4 flex items-center justify-between gap-4 border border-line-strong bg-plate px-[0.8rem] py-[0.6rem] font-mono text-xs leading-[1.4] tracking-[0.04em] text-ink-dim"
						role="status"
					>
						Development demo · fixture data and simulated actions
					</div>
				{/if}
				{#if app.error}
					<div
						class="mb-4 flex items-center justify-between gap-4 border border-s-2 border-scarlet bg-plate px-[0.8rem] py-[0.6rem] font-mono text-xs leading-[1.4] tracking-[0.04em] text-ink-dim"
						role="alert"
					>
						<span>{app.error}</span>
						<Button
							variant="destructive"
							size="sm"
							class="shrink-0 font-mono tracking-[0.1em] uppercase"
							onclick={() => void app.retryError()}
						>
							{app.errorRetry ? 'Retry' : 'Dismiss'}
						</Button>
					</div>
				{/if}
				<div class="sr-only" aria-live="polite">
					Execution target: {mode == null
						? 'Loading'
						: mode === 'remote'
							? 'Shadoword API'
							: mode === 'open_router'
								? 'OpenRouter'
								: 'Local machine'} · {poolSummary}
				</div>
				{@render children()}
			</div>
		</main>
	</div>
	<Toaster />
</div>
