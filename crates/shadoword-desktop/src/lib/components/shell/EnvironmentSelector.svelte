<script lang="ts">
	import { Cloud, Cpu, RadioTower } from '@lucide/svelte';
	import type { ServiceMode } from '$lib/bindings';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';
	import { tick } from 'svelte';

	const shell = useDesktopShell();
	const environments = [
		{ mode: 'local', label: 'Local', compact: 'Local', icon: Cpu },
		{ mode: 'remote', label: 'Shadoword API', compact: 'API', icon: RadioTower },
		{ mode: 'open_router', label: 'OpenRouter', compact: 'OpenRouter', icon: Cloud }
	] satisfies Array<{
		mode: ServiceMode;
		label: string;
		compact: string;
		icon: typeof Cpu;
	}>;

	const handleKeydown = async (event: KeyboardEvent, current: number) => {
		let next: number;
		if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (current + 1) % 3;
		else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (current + 2) % 3;
		else if (event.key === 'Home') next = 0;
		else if (event.key === 'End') next = 2;
		else return;
		event.preventDefault();
		await shell.selectEnvironment(environments[next].mode);
		await tick();
		document.querySelector<HTMLButtonElement>(`[data-environment="${shell.mode}"]`)?.focus();
	};
</script>

<div class="environment-selector">
	<span class="selector-label mono-micro">Execution target</span>
	<div class="environment-choices" role="radiogroup" aria-label="Execution target">
		{#each environments as environment, index (environment.mode)}
			{@const Icon = environment.icon}
			<button
				type="button"
				role="radio"
				class:active={shell.mode === environment.mode}
				aria-checked={shell.mode === environment.mode}
				tabindex={shell.mode === environment.mode || (shell.mode == null && index === 0) ? 0 : -1}
				data-environment={environment.mode}
				aria-label={environment.label}
				title={environment.label}
				disabled={shell.environmentLocked}
				onclick={() => shell.selectEnvironment(environment.mode)}
				onkeydown={(event) => void handleKeydown(event, index)}
			>
				<Icon size={15} strokeWidth={1.9} aria-hidden="true" />
				<span class="choice-label">{environment.compact}</span>
			</button>
		{/each}
	</div>
	<span class="switch-message mono-micro" aria-live="polite">
		{shell.environmentMessage}
	</span>
</div>

<style>
	.environment-selector {
		display: grid;
		gap: 0.45rem;
		border-bottom: 1px solid var(--line);
		padding: 0.65rem 0.85rem;
		background: var(--surface-0);
	}

	.selector-label {
		color: var(--ink-muted);
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.environment-choices {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 1px;
		border: 1px solid var(--line-strong);
		background: var(--line-strong);
	}

	button {
		display: grid;
		min-width: 0;
		min-height: 2.55rem;
		place-items: center;
		gap: 0.18rem;
		border: 0;
		padding: 0.42rem 0.2rem;
		background: var(--surface-1);
		color: var(--ink-muted);
		font: inherit;
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	button:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--ink);
	}

	button.active {
		box-shadow: inset 0 2px 0 var(--scarlet);
		background: var(--surface-2);
		color: var(--scarlet-lamp);
	}

	button:focus-visible {
		position: relative;
		z-index: 1;
		outline: 2px solid var(--ink);
		outline-offset: -2px;
	}

	button:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}

	.choice-label {
		overflow: hidden;
		max-width: 100%;
		font-family: var(--font-mono);
		font-size: 0.55rem;
		font-weight: 650;
		letter-spacing: -0.02em;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.switch-message {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}

	@media (max-width: 900px) {
		.environment-selector {
			padding: 0.55rem 0.45rem;
		}

		.selector-label,
		.choice-label {
			display: none;
		}

		.environment-choices {
			grid-template-columns: 1fr;
		}

		button {
			min-height: 2.75rem;
		}
	}
</style>
