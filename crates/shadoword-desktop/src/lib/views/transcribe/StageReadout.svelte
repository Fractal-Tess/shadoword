<script lang="ts">
	import { getTranscribeContext } from './context';

	const context = getTranscribeContext();
</script>

<div class="stage-readout" aria-live="polite">
	<div>
		<span class="mono-label">Target</span>
		<strong class="mono-caption">
			{context.mode === 'local' ? 'This machine' : context.endpointHost}
		</strong>
	</div>
	<div>
		<span class="mono-label">Model</span>
		<strong class="mono-caption">{context.modelName}</strong>
	</div>
	<div>
		<span class="mono-label">Delivery</span>
		<strong class="mono-caption"
			>{context.app.settings?.paste_method === 'direct'
				? 'Type directly'
				: context.app.settings?.copy_to_clipboard
					? 'Clipboard + surface'
					: 'Transcript surface'}</strong
		>
	</div>
</div>

<style>
	.stage-readout {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 1px;
		border-top: 1px solid var(--line);
		background: var(--line);
	}

	.stage-readout > div {
		display: grid;
		gap: 0.28rem;
		min-width: 0;
		padding: 0.7rem 0.85rem;
		background: var(--surface-1);
	}

	.stage-readout strong {
		overflow: hidden;
		color: var(--ink);
		font-weight: 400;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	@media (max-width: 860px) {
		.stage-readout {
			grid-template-columns: 1fr;
		}
	}
</style>
