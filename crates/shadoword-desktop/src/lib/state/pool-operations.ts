import { commands, type InferencePoolConfig } from '$lib/bindings';
import { errorMessage } from '$lib/display';
import {
	isStaleRuntimeError,
	runtimeWithInferencePool,
	validateInferencePoolCandidate
} from '$lib/inference-pool';
import type { DesktopStateContext } from './contracts';

export class PoolOperations {
	constructor(private app: DesktopStateContext) {}

	clearDraftFeedback() {
		this.app.poolApplyState = 'idle';
		this.app.poolFieldErrors = {};
		this.app.poolFeedback = null;
	}

	async applyDraft(pool: InferencePoolConfig) {
		if (this.app.poolMutationLocked)
			throw new Error(
				this.app.drainingPool
					? 'Wait for the draining generation to finish before reloading the pool.'
					: 'The runtime is busy. Wait for the current operation to finish.'
			);
		const runtime = this.app.overview?.runtime;
		if (!runtime) throw new Error('Refresh the runtime before applying an inference pool.');

		this.app.poolApplyState = 'applying';
		this.app.poolFeedback = 'Validating, loading units, and preparing the next generation…';
		try {
			const effective = await this.#validatedDraft(pool);
			await this.app.updateRuntime(runtimeWithInferencePool(runtime, effective));
			this.app.poolApplyState = 'applied';
			this.app.poolFeedback = `Generation ${this.app.overview?.runtime.generation ?? 'updated'} is active.`;
			return this.app.overview;
		} catch (error) {
			if (isStaleRuntimeError(error)) {
				this.app.poolApplyState = 'stale';
				await this.app.refreshOverview();
				this.app.poolFeedback =
					'The runtime changed elsewhere. Active state was refreshed; review the draft and retry.';
			} else {
				this.app.poolApplyState = 'failed';
				this.app.poolFeedback = `The active pool was kept unchanged. ${errorMessage(error)}`;
			}
			throw error;
		}
	}

	async #validatedDraft(pool: InferencePoolConfig) {
		const local = validateInferencePoolCandidate(pool);
		this.app.poolFieldErrors = local.fieldErrors;
		if (local.globalError) {
			this.app.poolFeedback = local.globalError;
			throw new Error(local.globalError);
		}
		return this.app.demo || this.app.settings?.mode === 'remote'
			? local.pool
			: await commands.validateLocalInferencePool(local.pool);
	}
}
