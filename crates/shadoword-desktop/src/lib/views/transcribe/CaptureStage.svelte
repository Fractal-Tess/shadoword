<script lang="ts">
	import CaptureCallout from './CaptureCallout.svelte';
	import CaptureControls from './CaptureControls.svelte';
	import StageReadout from './StageReadout.svelte';
	import { getTranscribeContext } from './context';

	const context = getTranscribeContext();
</script>

<section
	class:recording={context.app.recording}
	class:local={context.mode === 'local'}
	class:unavailable={context.captureBlocked}
	class="capture-stage"
	aria-labelledby="capture-title"
>
	<div class="spectrum" aria-hidden="true">
		<div class="bins"></div>
		<div class="bloom"></div>
		<div class="void"><span class="lance"></span></div>
	</div>

	<CaptureControls />

	{#if context.captureBlocked || context.app.processing}
		<CaptureCallout />
	{/if}

	<StageReadout />
</section>

<style>
	.capture-stage {
		position: relative;
		display: grid;
		grid-template-rows: 1fr auto;
		min-height: 0;
		overflow: hidden;
		border: 1px solid var(--line);
		background: var(--surface-0);
		isolation: isolate;
	}

	.spectrum {
		position: absolute;
		inset: 0;
		z-index: -1;
		mask-image: linear-gradient(
			to right,
			transparent 0%,
			rgb(0 0 0 / 0.04) 20%,
			rgb(0 0 0 / 0.42) 34%,
			rgb(0 0 0 / 1) 46%
		);
		opacity: 0.66;
	}

	.bins {
		position: absolute;
		inset: 0 auto 0 0;
		width: 160%;
		background-image:
			repeating-linear-gradient(
				90deg,
				transparent 0 5px,
				rgb(255 255 255 / 0.2) 5px 6px,
				transparent 6px 7px
			),
			repeating-linear-gradient(
				90deg,
				transparent 0 9px,
				rgb(255 255 255 / 0.1) 9px 10px,
				transparent 10px 11px
			),
			repeating-linear-gradient(
				90deg,
				transparent 0 21px,
				rgb(255 255 255 / 0.3) 21px 23px,
				transparent 23px 23px
			);
		mask-image: linear-gradient(
			to top,
			rgb(0 0 0 / 1) 0%,
			rgb(0 0 0 / 0.5) 24%,
			rgb(0 0 0 / 0.14) 56%,
			rgb(0 0 0 / 0) 86%
		);
	}

	.bloom {
		position: absolute;
		inset: 0;
		mix-blend-mode: screen;
		background-image:
			radial-gradient(5% 62% at 71% 96%, rgb(255 47 127 / 0.62) 0%, rgb(255 47 127 / 0) 100%),
			radial-gradient(4% 48% at 88% 96%, rgb(38 212 236 / 0.55) 0%, rgb(38 212 236 / 0) 100%),
			radial-gradient(6% 34% at 57% 100%, rgb(255 47 127 / 0.4) 0%, rgb(255 47 127 / 0) 100%),
			radial-gradient(3% 26% at 81% 100%, rgb(38 212 236 / 0.34) 0%, rgb(38 212 236 / 0) 100%);
	}

	.void {
		position: absolute;
		top: 0;
		bottom: 0;
		left: 49.64%;
		width: 3.26%;
		background: var(--surface-0);
		transform: scaleY(0);
		transform-origin: bottom;
		transition: transform 420ms cubic-bezier(0.16, 1, 0.3, 1);
	}

	.capture-stage.local .void {
		transform: scaleY(1);
	}

	.lance {
		position: absolute;
		top: 0;
		left: 50%;
		width: 1px;
		height: 74%;
		background: var(--scarlet);
	}

	.capture-stage.recording .bins {
		animation: bin-drift 2.6s linear infinite;
	}

	.capture-stage.recording .bloom {
		animation: bloom-breathe 4.3s ease-in-out infinite;
	}

	.capture-stage.unavailable .spectrum {
		opacity: 0.4;
	}

	@keyframes bin-drift {
		from {
			transform: translate3d(0, 0, 0);
		}
		to {
			transform: translate3d(-23px, 0, 0);
		}
	}

	@keyframes bloom-breathe {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.62;
		}
	}

	@media (max-width: 860px) {
		.spectrum {
			inset: auto 0 3.4rem;
			height: 6rem;
			mask-image: linear-gradient(
				to right,
				transparent 0%,
				rgb(0 0 0 / 0.2) 26%,
				rgb(0 0 0 / 1) 62%
			);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.capture-stage.recording .bins,
		.capture-stage.recording .bloom {
			animation: none;
		}

		.void {
			transition: none;
		}
	}
</style>
