<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { Minus, Square, X } from '@lucide/svelte';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import type { PageId } from '$lib/types';

	let { activePage }: { activePage: PageId } = $props();

	const minimize = async () => {
		if (!('__TAURI_INTERNALS__' in window)) return;
		await getCurrentWindow().minimize();
	};

	const toggleMaximize = async () => {
		if (!('__TAURI_INTERNALS__' in window)) return;
		await getCurrentWindow().toggleMaximize();
	};

	const close = async () => {
		if (!('__TAURI_INTERNALS__' in window)) return;
		await getCurrentWindow().close();
	};
</script>

<header class="window-chrome" data-tauri-drag-region>
	<div class="window-identity" data-tauri-drag-region>
		<BrandMark compact />
		<span data-tauri-drag-region>Shadoword</span>
		<i aria-hidden="true" data-tauri-drag-region></i>
		<strong data-tauri-drag-region>{activePage}</strong>
	</div>
	<div class="drag-field" data-tauri-drag-region aria-hidden="true"></div>
	<div class="window-controls">
		<button type="button" aria-label="Minimize window" title="Minimize" onclick={minimize}>
			<Minus aria-hidden="true" />
		</button>
		<button type="button" aria-label="Maximize window" title="Maximize" onclick={toggleMaximize}>
			<Square aria-hidden="true" />
		</button>
		<button class="close" type="button" aria-label="Close window" title="Close" onclick={close}>
			<X aria-hidden="true" />
		</button>
	</div>
</header>

<style>
	.window-chrome {
		display: grid;
		grid-template-columns: auto minmax(2rem, 1fr) auto;
		grid-column: 1 / -1;
		grid-row: 1;
		align-items: stretch;
		min-width: 0;
		border-bottom: 1px solid var(--line);
		background: var(--surface-1);
		user-select: none;
	}

	.window-identity {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		min-width: 0;
		padding: 0 1rem;
		color: var(--ink-muted);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.window-identity :global(.brand) {
		color: var(--scarlet);
	}

	.window-identity > span {
		color: var(--ink);
		font-weight: 620;
		letter-spacing: 0.14em;
	}

	.window-identity i {
		width: 1px;
		height: 0.85rem;
		background: var(--line-strong);
	}

	.window-identity strong {
		overflow: hidden;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.drag-field {
		min-width: 0;
	}

	.window-controls {
		display: grid;
		grid-template-columns: repeat(3, 2.75rem);
	}

	.window-controls button {
		display: grid;
		place-items: center;
		border: 0;
		border-left: 1px solid var(--line);
		background: transparent;
		color: var(--ink-muted);
		cursor: pointer;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}

	.window-controls button:hover {
		background: var(--surface-2);
		color: var(--ink);
	}

	.window-controls button.close:hover {
		background: var(--scarlet);
		color: var(--on-scarlet);
	}

	.window-controls :global(svg) {
		width: 1.05rem;
		height: 1.05rem;
		stroke-width: 1.8;
	}

	@media (max-width: 900px) {
		.window-identity strong,
		.window-identity i {
			display: none;
		}
	}
</style>
