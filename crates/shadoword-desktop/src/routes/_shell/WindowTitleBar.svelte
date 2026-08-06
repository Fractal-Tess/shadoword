<script lang="ts">
	import { Minus, Square, X } from '@lucide/svelte';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';

	const shell = useDesktopShell();
</script>

<header
	class="relative z-50 flex h-8 shrink-0 items-stretch border-b border-line bg-night text-ink-muted select-none"
	aria-label="Window title bar"
>
	<div data-tauri-drag-region class="min-w-0 flex-1"></div>
	<div class="flex shrink-0" aria-label="Window controls">
		<button
			type="button"
			class="grid w-10 cursor-pointer place-items-center border-0 bg-transparent text-ink-muted transition-colors hover:bg-raised hover:text-ink focus-visible:relative focus-visible:z-1 focus-visible:-outline-offset-2"
			onclick={() => void shell.minimizeWindow()}
			aria-label="Minimize window"
			title="Minimize"
		>
			<Minus size={14} strokeWidth={1.8} aria-hidden="true" />
		</button>
		<button
			type="button"
			class="grid w-10 cursor-pointer place-items-center border-0 bg-transparent text-ink-muted transition-colors hover:bg-raised hover:text-ink focus-visible:relative focus-visible:z-1 focus-visible:-outline-offset-2"
			onclick={() => void shell.toggleMaximizeWindow()}
			aria-label="Maximize or restore window"
			title="Maximize or restore"
		>
			<Square size={11} strokeWidth={1.8} aria-hidden="true" />
		</button>
		<button
			type="button"
			class="grid w-10 cursor-pointer place-items-center border-0 bg-transparent text-ink-muted transition-colors hover:bg-scarlet hover:text-on-scarlet focus-visible:relative focus-visible:z-1 focus-visible:-outline-offset-2"
			onclick={() => void shell.requestWindowClose()}
			aria-label="Close window"
			title="Close"
		>
			<X size={14} strokeWidth={1.8} aria-hidden="true" />
		</button>
	</div>
</header>
