<script lang="ts">
	import { getTranscribeContext } from './state.svelte';

	const context = getTranscribeContext();
</script>

<div
	class="grid grid-cols-3 gap-px border-t border-line bg-line [@media(max-width:860px)]:grid-cols-1"
	aria-live="polite"
>
	<div class="grid min-w-0 gap-[0.28rem] bg-plate px-[0.85rem] py-[0.7rem]">
		<span
			class="font-mono text-xs leading-[1.3] font-medium tracking-[0.14em] text-ink-muted uppercase"
			>Target</span
		>
		<strong
			class="overflow-hidden font-mono text-[0.8125rem] leading-normal font-normal text-ellipsis whitespace-nowrap text-ink"
		>
			{context.mode === 'local' ? 'This machine' : context.endpointHost}
		</strong>
	</div>
	<div class="grid min-w-0 gap-[0.28rem] bg-plate px-[0.85rem] py-[0.7rem]">
		<span
			class="font-mono text-xs leading-[1.3] font-medium tracking-[0.14em] text-ink-muted uppercase"
			>Model</span
		>
		<strong
			class="overflow-hidden font-mono text-[0.8125rem] leading-normal font-normal text-ellipsis whitespace-nowrap text-ink"
			>{context.modelName}</strong
		>
	</div>
	<div class="grid min-w-0 gap-[0.28rem] bg-plate px-[0.85rem] py-[0.7rem]">
		<span
			class="font-mono text-xs leading-[1.3] font-medium tracking-[0.14em] text-ink-muted uppercase"
			>Delivery</span
		>
		<strong
			class="overflow-hidden font-mono text-[0.8125rem] leading-normal font-normal text-ellipsis whitespace-nowrap text-ink"
			>{context.app.settings?.paste_method === 'direct'
				? 'Type directly'
				: context.app.settings?.copy_to_clipboard
					? 'Clipboard + surface'
					: 'Transcript surface'}</strong
		>
	</div>
</div>
