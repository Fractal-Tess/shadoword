<script lang="ts">
	import CircleCheckIcon from '@lucide/svelte/icons/circle-check';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import OctagonXIcon from '@lucide/svelte/icons/octagon-x';
	import XIcon from '@lucide/svelte/icons/x';
	import { Toaster as Sonner, type ToasterProps as SonnerProps } from 'svelte-sonner';

	let { ...restProps }: SonnerProps = $props();
</script>

<Sonner
	theme="dark"
	position="top-right"
	duration={4200}
	visibleToasts={4}
	gap={8}
	closeButton
	offset={{ top: '2.5rem', right: '1rem' }}
	mobileOffset={{ top: '2.5rem', right: '1rem', left: '1rem' }}
	toastOptions={{
		unstyled: true,
		classes: {
			toast:
				'grid w-[min(24rem,calc(100vw-2rem))] grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 border border-line-strong bg-[color-mix(in_srgb,var(--surface-1)_96%,transparent)] p-3 shadow-[0_1rem_2.5rem_rgb(0_0_0/0.38)] outline-none data-[type=error]:border-scarlet data-[type=error]:[&_[data-icon]]:bg-scarlet data-[type=error]:[&_[data-icon]]:text-on-scarlet focus-visible:ring-2 focus-visible:ring-ink/40',
			icon: 'col-start-1 row-start-1 grid size-8 place-items-center bg-ink text-night [&_svg]:size-[0.9375rem]',
			content: 'col-start-2 row-start-1 grid min-w-0 gap-[0.18rem]',
			title: 'font-mono text-xs font-bold text-ink',
			description: 'font-mono text-[0.6875rem] leading-[1.45] break-words text-ink-muted',
			closeButton:
				'col-start-3 row-start-1 grid size-7 cursor-pointer place-items-center border-0 bg-transparent text-ink-muted outline-none hover:bg-raised hover:text-ink focus-visible:ring-2 focus-visible:ring-ink/40 [&_svg]:size-3.5'
		}
	}}
	{...restProps}
>
	{#snippet loadingIcon()}
		<Loader2Icon class="animate-spin motion-reduce:animate-none" />
	{/snippet}
	{#snippet successIcon()}
		<CircleCheckIcon />
	{/snippet}
	{#snippet errorIcon()}
		<OctagonXIcon />
	{/snippet}
	{#snippet closeIcon()}
		<XIcon />
	{/snippet}
</Sonner>
