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
	import { resolve } from '$app/paths';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import { PAGE_ROUTES } from '$lib/shell/routes';
	import type { PageId } from '$lib/types';
	import { cn } from '$lib/utils';

	const shell = useDesktopShell();
	const destinationIcons = {
		transcribe: Mic2,
		models: Box,
		history: Clock3,
		settings: Settings2,
		capture: AudioLines,
		transcription: Captions,
		output: Send,
		application: AppWindow,
		about: Info
	} satisfies Record<PageId, typeof Mic2>;
	const destinations = PAGE_ROUTES.map((route) => ({
		...route,
		icon: destinationIcons[route.id]
	}));
	const navigationItemClass = cn(
		'relative grid min-h-[2.35rem] w-full grid-cols-[1.25rem_1fr] items-center gap-3 px-[1.15rem] text-left text-ink-muted no-underline',
		'transition-colors duration-[120ms] ease-linear hover:bg-raised hover:text-ink focus-visible:-outline-offset-2',
		'aria-[current=page]:bg-raised aria-[current=page]:text-ink',
		'max-[999px]:grid-cols-[1.25rem] max-[999px]:justify-center max-[999px]:px-0'
	);

	function handleKeydown(event: KeyboardEvent, page: PageId) {
		if (event.key !== ' ') return;
		event.preventDefault();
		void shell.navigate(page);
	}
</script>

<nav class="grid gap-px py-2" aria-label="Primary navigation">
	{#each destinations.filter( (destination) => shell.isPageAvailable(destination.id) ) as destination (destination.id)}
		{@const Icon = destination.icon}
		<a
			href={resolve(shell.hrefFor(destination.id))}
			class={navigationItemClass}
			onkeydown={(event) => handleKeydown(event, destination.id)}
			aria-current={shell.activePage === destination.id ? 'page' : undefined}
			aria-label={destination.label}
		>
			{#if shell.activePage === destination.id}
				<span class="absolute inset-y-0 left-0 w-0.5 bg-scarlet" aria-hidden="true"></span>
			{/if}
			<Icon size={20} strokeWidth={1.8} aria-hidden="true" />
			<span
				class="truncate font-display text-lg leading-none tracking-[0.035em] uppercase max-[999px]:hidden"
			>
				{destination.label}
			</span>
		</a>
	{/each}
</nav>
