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
		this.app.poolValidationState = 'idle';
		this.app.poolApplyState = 'idle';
		this.app.poolFieldErrors = {};
		this.app.poolFeedback = null;
	}

	async validateDraft(pool: InferencePoolConfig) {
		if (this.app.captureLocked) throw new Error('Finish the active recording before validation.');
		const local = validateInferencePoolCandidate(pool);
		this.app.poolFieldErrors = local.fieldErrors;
		if (local.globalError) {
			this.app.poolValidationState = 'invalid';
			this.app.poolFeedback = local.globalError;
			throw new Error(local.globalError);
		}

		this.app.poolValidationState = 'validating';
		this.app.poolApplyState = 'idle';
		this.app.poolFeedback = 'Checking execution targets and resource limits…';
		try {
			const effective =
				this.app.demo || this.app.settings?.mode === 'remote'
					? local.pool
					: await commands.validateLocalInferencePool(local.pool);
			this.app.poolValidationState = 'valid';
			this.app.poolFeedback =
				this.app.settings?.mode === 'local'
					? 'Pool is valid for the detected local hardware.'
					: 'Pool shape is valid. The remote host will verify hardware during apply.';
			return effective;
		} catch (error) {
			this.app.poolValidationState = 'invalid';
			this.app.poolFeedback = errorMessage(error);
			throw error;
		}
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
			const effective = await this.validateDraft(pool);
			this.app.poolApplyState = 'applying';
			await this.app.updateRuntime(runtimeWithInferencePool(runtime, effective));
			this.app.poolApplyState = 'applied';
			this.app.poolValidationState = 'valid';
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
}
