<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';
	import type { ClassValue } from 'svelte/elements';
	import { createBlinkingSquaresAttachment, type Direction } from './blinking-squares.svelte';

	let {
		class: className,
		fill = false,
		active = true,
		children,
		direction = 'right',
		gridSize = 52,
		squareColor = '#e6202c',
		backgroundColor = '#07090d',
		falloff = 1.6,
		fadeStart = 0.08,
		fadeEnd = 1,
		squareSize = 0.5,
		minBrightness = 0.3,
		twinkleSpeed = 0.45,
		twinkleStrength = 0.35,
		intensity = 0.65,
		opacity = 0.42,
		dpr = 1.25
	}: {
		class?: ClassValue;
		fill?: boolean;
		active?: boolean;
		children?: Snippet;
		direction?: Direction;
		gridSize?: number;
		squareColor?: string;
		backgroundColor?: string;
		falloff?: number;
		fadeStart?: number;
		fadeEnd?: number;
		squareSize?: number;
		minBrightness?: number;
		twinkleSpeed?: number;
		twinkleStrength?: number;
		intensity?: number;
		opacity?: number;
		dpr?: number;
	} = $props();

	const renderSquares = createBlinkingSquaresAttachment(() => ({
		active,
		direction,
		gridSize,
		squareColor,
		backgroundColor,
		falloff,
		fadeStart,
		fadeEnd,
		squareSize,
		minBrightness,
		twinkleSpeed,
		twinkleStrength,
		intensity,
		opacity,
		dpr
	}));
</script>

<div
	class={cn(
		'size-full overflow-hidden transition-opacity duration-[850ms] ease-[cubic-bezier(0.22,1,0.36,1)] motion-reduce:transition-none',
		fill ? 'absolute inset-0' : 'relative',
		active ? 'opacity-100 will-change-[opacity]' : 'opacity-0',
		className
	)}
>
	<canvas
		class="pointer-events-none absolute inset-0 block size-full"
		{@attach renderSquares}
		aria-hidden="true"
	></canvas>
	{#if children}
		<div class="relative z-1">{@render children()}</div>
	{/if}
</div>
