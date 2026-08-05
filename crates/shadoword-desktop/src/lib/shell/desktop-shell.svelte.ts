import { replaceState } from '$app/navigation';
import { resolve } from '$app/paths';
import type { ServiceMode } from '$lib/bindings';
import type { DesktopAppState } from '$lib/app-state.svelte';
import type { PageId } from '$lib/types';
import { fallbackPage, isPageAvailable } from './environment-policy';
import { tick } from 'svelte';
import { SvelteURL } from 'svelte/reactivity';

export type EnvironmentSwitchState = 'idle' | 'switching' | 'failed';

export class DesktopShellState {
	readonly app: DesktopAppState;
	activePage = $state<PageId>('transcribe');
	environmentSwitchState = $state<EnvironmentSwitchState>('idle');
	environmentMessage = $state('');

	constructor(app: DesktopAppState, initialPage: PageId) {
		this.app = app;
		this.activePage = initialPage;
	}

	get mode() {
		return this.app.settings?.mode ?? null;
	}

	get environmentLocked() {
		return (
			!this.app.settings ||
			this.app.captureLocked ||
			this.app.activity === 'busy' ||
			this.environmentSwitchState === 'switching'
		);
	}

	isPageAvailable(page: PageId) {
		return isPageAvailable(page, this.mode);
	}

	async selectEnvironment(mode: ServiceMode) {
		if (this.environmentLocked || this.mode === mode) return;
		this.environmentSwitchState = 'switching';
		this.environmentMessage = `Switching to ${environmentLabel(mode)}…`;
		try {
			await this.app.setMode(mode);
			if (this.app.settings?.mode !== mode) {
				this.environmentSwitchState = 'failed';
				this.environmentMessage = `Could not switch to ${environmentLabel(mode)}`;
				return;
			}
			this.environmentSwitchState = 'idle';
			this.environmentMessage = `${environmentLabel(mode)} selected`;
			await this.reconcilePage();
		} catch {
			this.environmentSwitchState = 'failed';
			this.environmentMessage = `Could not switch to ${environmentLabel(mode)}`;
		}
	}

	async navigate(page: PageId) {
		if (!this.isPageAvailable(page)) return;
		this.activePage = page;
		this.syncPageQuery(page);
		await this.focusWorkSurface();
	}

	async reconcilePage() {
		const next = fallbackPage(this.activePage, this.mode);
		if (next !== this.activePage) await this.navigate(next);
	}

	private syncPageQuery(page: PageId) {
		if (typeof window === 'undefined') return;
		const url = new SvelteURL(window.location.href);
		if (page === 'transcribe') url.searchParams.delete('page');
		else url.searchParams.set('page', page);
		const query = url.searchParams.toString();
		const target: '/' | `/?${string}` = query ? `/?${query}` : '/';
		replaceState(resolve(target), {});
	}

	private async focusWorkSurface() {
		await tick();
		const main = document.querySelector<HTMLElement>('.app-shell > main');
		main?.scrollTo({ top: 0 });
		main?.focus({ preventScroll: true });
	}
}

export function environmentLabel(mode: ServiceMode) {
	if (mode === 'local') return 'Local';
	if (mode === 'open_router') return 'OpenRouter';
	return 'Shadoword API';
}
