import type { DesktopAppState } from '$lib/app-state.svelte';
import {
	commands,
	type ApiTokenRole,
	type ApiTokenSummaryDto,
	type CreatedApiTokenDto
} from '$lib/bindings';
import { errorMessage } from '$lib/display';

const demoTokens: ApiTokenSummaryDto[] = [
	{ name: 'workstation', role: 'admin' },
	{ name: 'laptop', role: 'user' }
];

/**
 * Token management is deliberately not part of the settings form: it acts on the
 * daemon the moment a button is pressed, so it has no draft state, no dirty
 * tracking, and nothing for the autosave to schedule.
 */
export class RemoteTokenSettingsState {
	tokens = $state.raw<ApiTokenSummaryDto[]>([]);
	loaded = $state(false);
	busy = $state(false);
	error = $state('');
	name = $state('');
	role = $state<ApiTokenRole>('user');
	/**
	 * Held until the operator dismisses it. The daemon stores only a hash, so
	 * clearing this before it has been copied loses the token for good.
	 */
	issued = $state.raw<CreatedApiTokenDto | null>(null);
	copied = $state(false);
	#app: DesktopAppState;

	constructor(app: DesktopAppState) {
		this.#app = app;
	}

	get canCreate() {
		return !this.busy && this.name.trim() !== '';
	}

	async load() {
		if (this.busy) return;
		this.busy = true;
		this.error = '';
		try {
			this.tokens = this.#app.demo ? demoTokens : await commands.listRemoteTokens();
			this.loaded = true;
		} catch (error) {
			this.error = errorMessage(error);
		} finally {
			this.busy = false;
		}
	}

	async create() {
		const name = this.name.trim();
		if (this.busy || !name) return;
		this.busy = true;
		this.error = '';
		try {
			this.issued = this.#app.demo
				? { name, role: this.role, token: `swd_${this.role}_demo-token-not-valid` }
				: await commands.createRemoteToken({ name, role: this.role });
			this.copied = false;
			this.name = '';
			this.tokens = this.#app.demo
				? [...this.tokens, { name, role: this.role }]
				: await commands.listRemoteTokens();
		} catch (error) {
			this.error = errorMessage(error);
		} finally {
			this.busy = false;
		}
	}

	async revoke(name: string) {
		if (this.busy) return;
		this.busy = true;
		this.error = '';
		try {
			this.tokens = this.#app.demo
				? this.tokens.filter((token) => token.name !== name)
				: await commands.revokeRemoteToken(name);
			if (this.issued?.name === name) this.dismissIssued();
		} catch (error) {
			this.error = errorMessage(error);
		} finally {
			this.busy = false;
		}
	}

	async copyIssued() {
		if (!this.issued) return;
		await navigator.clipboard?.writeText(this.issued.token);
		this.copied = true;
	}

	dismissIssued() {
		this.issued = null;
		this.copied = false;
	}
}
