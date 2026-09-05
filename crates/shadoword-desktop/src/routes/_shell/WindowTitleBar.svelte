<script lang="ts">
	import { Minus, Square, X } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { useDesktopShell } from '$lib/shell/desktop-shell-context';

	const shell = useDesktopShell();
</script>

<header
	class="relative z-50 flex h-8 shrink-0 items-stretch border-b border-line bg-night text-ink-muted select-none"
	aria-label="Window title bar"
>
	<div data-tauri-drag-region class="min-w-0 flex-1"></div>
	<div class="flex shrink-0" aria-label="Window controls">
		<Button
			variant="ghost"
			size="icon"
			class="h-full w-10 rounded-none"
			onclick={() => void shell.minimizeWindow()}
			aria-label="Minimize window"
			title="Minimize"
		>
			<Minus strokeWidth={1.8} aria-hidden="true" />
		</Button>
		<Button
			variant="ghost"
			size="icon"
			class="h-full w-10 rounded-none"
			onclick={() => void shell.toggleMaximizeWindow()}
			aria-label="Maximize or restore window"
			title="Maximize or restore"
		>
			<Square strokeWidth={1.8} aria-hidden="true" />
		</Button>
		<Button
			variant="destructive"
			size="icon"
			class="h-full w-10 rounded-none"
			onclick={() => void shell.requestWindowClose()}
			aria-label="Close window"
			title="Close"
		>
			<X strokeWidth={1.8} aria-hidden="true" />
		</Button>
	</div>
</header>
