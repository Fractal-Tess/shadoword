<script lang="ts">
	import { Switch as SwitchPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		checked = $bindable(false),
		size = 'default',
		...restProps
	}: WithoutChildrenOrChild<SwitchPrimitive.RootProps> & {
		size?: 'sm' | 'default';
	} = $props();
</script>

<SwitchPrimitive.Root
	bind:ref
	bind:checked
	data-slot="switch"
	data-size={size}
	class={cn(
		'peer group/switch relative inline-flex h-6 w-14 shrink-0 items-center border border-line-strong bg-night p-0.5 transition-colors outline-none after:absolute after:-inset-x-2 after:-inset-y-2 focus-visible:border-ink focus-visible:ring-2 focus-visible:ring-ink/30 aria-invalid:border-scarlet aria-invalid:ring-2 aria-invalid:ring-scarlet/20 data-checked:border-scarlet data-checked:bg-scarlet-deep data-disabled:cursor-not-allowed data-disabled:opacity-50',
		className
	)}
	{...restProps}
>
	<span
		class={cn(
			'absolute end-[0.32rem] font-mono text-[0.5rem] leading-none font-bold tracking-[0.04em] text-ink-muted',
			checked && 'start-[0.32rem] end-auto text-on-scarlet'
		)}
		aria-hidden="true"
	>
		{checked ? 'ON' : 'OFF'}
	</span>
	<SwitchPrimitive.Thumb
		data-slot="switch-thumb"
		class="pointer-events-none relative z-10 block size-[18px] bg-ink ring-0 transition-transform motion-reduce:transition-none data-checked:translate-x-[30px] dark:data-checked:bg-on-scarlet data-unchecked:translate-x-0"
	/>
</SwitchPrimitive.Root>
