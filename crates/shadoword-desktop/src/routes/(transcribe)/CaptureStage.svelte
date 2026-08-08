<script lang="ts">
	import CaptureCallout from './CaptureCallout.svelte';
	import CaptureControls from './CaptureControls.svelte';
	import StageReadout from './StageReadout.svelte';
	import { cn } from '$lib/utils';
	import { getTranscribeContext } from './context';

	const context = getTranscribeContext();
</script>

<section
	class="relative isolate grid min-h-0 grid-rows-[1fr_auto] overflow-hidden border border-line bg-night"
	aria-labelledby="capture-title"
>
	<div
		class={cn(
			'absolute inset-0 -z-1 [mask-image:linear-gradient(to_right,transparent_0%,rgb(0_0_0/0.04)_16%,rgb(0_0_0/0.42)_32%,rgb(0_0_0/1)_46%,rgb(0_0_0/1)_78%,rgb(0_0_0/0.42)_88%,rgb(0_0_0/0.04)_97%,transparent_100%)] opacity-66 [@media(max-width:860px)]:inset-x-0 [@media(max-width:860px)]:top-auto [@media(max-width:860px)]:bottom-[3.4rem] [@media(max-width:860px)]:h-24 [@media(max-width:860px)]:[mask-image:linear-gradient(to_right,transparent_0%,rgb(0_0_0/0.2)_26%,rgb(0_0_0/1)_46%,rgb(0_0_0/1)_78%,rgb(0_0_0/0.2)_92%,transparent_100%)]',
			context.captureBlocked && 'opacity-40'
		)}
		aria-hidden="true"
	>
		<div
			class={cn(
				'absolute inset-y-0 left-0 w-[160%] [background-image:repeating-linear-gradient(90deg,transparent_0_5px,rgb(255_255_255/0.2)_5px_6px,transparent_6px_7px),repeating-linear-gradient(90deg,transparent_0_9px,rgb(255_255_255/0.1)_9px_10px,transparent_10px_11px),repeating-linear-gradient(90deg,transparent_0_21px,rgb(255_255_255/0.3)_21px_23px,transparent_23px_23px)] [mask-image:linear-gradient(to_top,rgb(0_0_0/1)_0%,rgb(0_0_0/0.5)_24%,rgb(0_0_0/0.14)_56%,rgb(0_0_0/0)_86%)]',
				'motion-reduce:animate-none',
				context.app.recording
					? 'animate-[bin-drift_0.65s_linear_infinite]'
					: 'animate-[bin-drift_2.6s_linear_infinite]'
			)}
		></div>
		<div
			class={cn(
				'absolute inset-0 bg-[radial-gradient(5%_62%_at_55%_96%,color-mix(in_srgb,var(--scarlet-lamp)_62%,transparent)_0%,transparent_100%),radial-gradient(4%_48%_at_69%_96%,color-mix(in_srgb,var(--aqua)_55%,transparent)_0%,transparent_100%),radial-gradient(6%_34%_at_48%_100%,color-mix(in_srgb,var(--scarlet-lamp)_40%,transparent)_0%,transparent_100%),radial-gradient(3%_26%_at_76%_100%,color-mix(in_srgb,var(--aqua)_34%,transparent)_0%,transparent_100%)] mix-blend-screen',
				'motion-reduce:animate-none',
				context.app.recording
					? 'animate-[bloom-breathe_1.075s_ease-in-out_infinite]'
					: 'animate-[bloom-breathe_4.3s_ease-in-out_infinite]'
			)}
		></div>
		<div
			class={cn(
				'absolute top-0 bottom-0 left-[49.64%] w-[3.26%] origin-bottom bg-night transition-transform duration-500 ease-[cubic-bezier(0.16,1,0.3,1)] motion-reduce:transition-none',
				context.mode === 'local' ? '[transform:scaleY(1)]' : '[transform:scaleY(0)]'
			)}
		>
			<span class="absolute top-0 left-1/2 h-[74%] w-px bg-scarlet"></span>
		</div>
	</div>

	<CaptureControls />

	{#if context.captureBlocked}
		<CaptureCallout />
	{/if}

	<StageReadout />
</section>
