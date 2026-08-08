<script lang="ts">
	import { Cpu, RadioTower } from '@lucide/svelte';
	import type { ServiceMode } from '$lib/bindings';
	import OpenRouterIcon from '$lib/components/icons/OpenRouterIcon.svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import { cn } from '$lib/utils';
	import { tick } from 'svelte';

	const shell = useDesktopShell();
	const environments = [
		{ mode: 'local', label: 'Local', lines: ['Local'] },
		{ mode: 'remote', label: 'Shadoword API', lines: ['Shadoword', 'API'] },
		{ mode: 'open_router', label: 'OpenRouter', lines: ['Open', 'Router'] }
	] satisfies Array<{
		mode: ServiceMode;
		label: string;
		lines: string[];
	}>;
	const environmentButtonClass = cn(
		'grid min-h-16 min-w-0 cursor-pointer place-items-center gap-1 border-0 bg-plate px-1 py-2 font-[inherit] text-ink-muted',
		'transition-colors duration-[120ms] ease-linear hover:not-disabled:bg-raised hover:not-disabled:text-ink focus-visible:relative focus-visible:z-[1] focus-visible:-outline-offset-2',
		'disabled:cursor-not-allowed disabled:opacity-50 aria-checked:bg-raised aria-checked:text-scarlet-lamp aria-checked:shadow-[inset_0_2px_0_var(--scarlet)] aria-checked:hover:not-disabled:text-scarlet-lamp',
		'max-[999px]:min-h-[4.5rem]'
	);

	const handleKeydown = async (event: KeyboardEvent, current: number) => {
		let next: number;
		if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
			next = (current + 1) % environments.length;
		} else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
			next = (current + environments.length - 1) % environments.length;
		} else if (event.key === 'Home') next = 0;
		else if (event.key === 'End') next = environments.length - 1;
		else return;
		event.preventDefault();
		await shell.selectEnvironment(environments[next].mode);
		await tick();
		document.querySelector<HTMLButtonElement>(`[data-environment="${shell.mode}"]`)?.focus();
	};
</script>

<div class="grid border-b border-line bg-night">
	<div
		class="grid w-full grid-cols-3 gap-px border border-line-strong bg-line-strong max-[999px]:grid-cols-1"
		role="radiogroup"
		aria-label="Execution target"
	>
		{#each environments as environment, index (environment.mode)}
			<button
				type="button"
				role="radio"
				class={environmentButtonClass}
				aria-checked={shell.mode === environment.mode}
				tabindex={shell.mode === environment.mode || (shell.mode == null && index === 0) ? 0 : -1}
				data-environment={environment.mode}
				aria-label={environment.label}
				title={environment.label}
				disabled={shell.environmentLocked}
				onclick={() => shell.selectEnvironment(environment.mode)}
				onkeydown={(event) => void handleKeydown(event, index)}
			>
				{#if environment.mode === 'local'}
					<Cpu size={18} strokeWidth={1.9} aria-hidden="true" />
				{:else if environment.mode === 'remote'}
					<RadioTower size={18} strokeWidth={1.9} aria-hidden="true" />
				{:else}
					<OpenRouterIcon size={19} title="" />
				{/if}
				<span
					class="grid max-w-full text-center font-mono text-[0.6875rem] leading-[1.1] font-[650] tracking-[-0.025em]"
				>
					{#each environment.lines as line (line)}<span>{line}</span>{/each}
				</span>
			</button>
		{/each}
	</div>
	<span class="sr-only" aria-live="polite">
		{shell.environmentMessage}
	</span>
</div>
