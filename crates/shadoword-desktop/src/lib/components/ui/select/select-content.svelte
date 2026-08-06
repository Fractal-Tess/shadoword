<script lang="ts">
	import { Select as SelectPrimitive } from 'bits-ui';
	import { cn, type WithoutChild } from '$lib/utils.js';
	import type { WithoutChildrenOrChild } from '$lib/utils.js';
	import SelectPortal from './select-portal.svelte';
	import SelectScrollDownButton from './select-scroll-down-button.svelte';
	import SelectScrollUpButton from './select-scroll-up-button.svelte';
	import type { ComponentProps } from 'svelte';

	let {
		ref = $bindable(null),
		class: className,
		sideOffset = 4,
		portalProps,
		children,
		preventScroll = true,
		...restProps
	}: WithoutChild<SelectPrimitive.ContentProps> & {
		portalProps?: WithoutChildrenOrChild<ComponentProps<typeof SelectPortal>>;
	} = $props();
</script>

<SelectPortal {...portalProps}>
	<SelectPrimitive.Content
		bind:ref
		{sideOffset}
		{preventScroll}
		data-slot="select-content"
		class={cn(
			'relative isolate z-50 max-h-80 min-w-44 overflow-x-hidden overflow-y-auto border border-line-strong bg-plate text-ink shadow-[0_12px_32px_rgb(0_0_0/0.42)] duration-100 data-[side=bottom]:slide-in-from-top-1 data-[side=inline-end]:slide-in-from-left-1 data-[side=inline-start]:slide-in-from-right-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1 motion-reduce:animate-none data-open:animate-in data-open:fade-in-0 data-closed:animate-out data-closed:fade-out-0',
			className
		)}
		{...restProps}
	>
		<SelectScrollUpButton />
		<SelectPrimitive.Viewport
			class={cn('w-full min-w-(--bits-select-anchor-width) scroll-my-1 p-1')}
		>
			{@render children?.()}
		</SelectPrimitive.Viewport>
		<SelectScrollDownButton />
	</SelectPrimitive.Content>
</SelectPortal>
