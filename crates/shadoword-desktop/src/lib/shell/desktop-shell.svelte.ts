import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { page } from '$app/state';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { DesktopAppState } from '$lib/app-state.svelte';
import type { ServiceMode } from '$lib/bindings';
import { errorMessage } from '$lib/display';
import type { PageId } from '$lib/types';
import { tick } from 'svelte';
import { SvelteURL } from 'svelte/reactivity';
import { fallbackPage, isPageAvailable } from './environment-policy';
import {
	navigationKey,
	pageHref,
	pageIdFromLegacyValue,
	pageIdFromPathname,
	pageUrl
} from './routes';

export type EnvironmentSwitchState = 'idle' | 'switching' | 'failed';

type NavigationOptions = {
	flushSettings: boolean;
	replaceState: boolean;
};

export class DesktopShellState {
	readonly app: DesktopAppState;
	environmentSwitchState = $state<EnvironmentSwitchState>('idle');
	environmentMessage = $state('');
	#flushPendingSettings: (() => Promise<void>) | null = null;
	#approvedNavigation: string | null = null;
	#navigationQueue = Promise.resolve();
	#workSurface: HTMLElement | null = null;

	workSurfaceAttachment = (element: HTMLElement) => {
		this.#workSurface = element;
		return () => {
			if (this.#workSurface === element) this.#workSurface = null;
		};
	};

	constructor(app: DesktopAppState) {
		this.app = app;
	}

	get activePage() {
		return pageIdFromPathname(page.url.pathname);
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

	hrefFor(pageId: PageId) {
		return pageHref(pageId, page.url);
	}

	async selectEnvironment(mode: ServiceMode) {
		if (this.environmentLocked || this.mode === mode) return;
		try {
			await this.flushSettings();
		} catch {
			this.environmentSwitchState = 'failed';
			this.environmentMessage = 'Save pending settings before switching targets';
			return;
		}
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

	navigate(pageId: PageId) {
		if (!this.isPageAvailable(pageId)) return Promise.resolve();
		return this.#navigateTo(pageUrl(pageId, page.url), {
			flushSettings: true,
			replaceState: false
		});
	}

	shouldGuardNavigation(target: URL) {
		const targetKey = navigationKey(target);
		if (this.#approvedNavigation === targetKey) {
			this.#approvedNavigation = null;
			return false;
		}
		return this.#flushPendingSettings !== null && targetKey !== navigationKey(page.url);
	}

	continueGuardedNavigation(target: URL, popstateDelta: number | null) {
		if (popstateDelta != null) return this.#resumePopstate(target, popstateDelta);
		return this.#navigateTo(target, { flushSettings: true, replaceState: false });
	}

	registerSettingsFlush(flush: () => Promise<void>) {
		this.#flushPendingSettings = flush;
		return () => {
			if (this.#flushPendingSettings === flush) this.#flushPendingSettings = null;
		};
	}

	flushSettings() {
		return this.#flushPendingSettings?.() ?? Promise.resolve();
	}

	async canonicalizeLegacyRoute() {
		if (page.url.pathname !== '/' || !page.url.searchParams.has('page')) return;
		const targetPage = pageIdFromLegacyValue(page.url.searchParams.get('page'));
		await this.#navigateTo(pageUrl(targetPage, page.url), {
			flushSettings: false,
			replaceState: true
		});
	}

	async reconcilePage() {
		const next = fallbackPage(this.activePage, this.mode);
		if (next === this.activePage) return;
		await this.#navigateTo(pageUrl(next, page.url), {
			flushSettings: true,
			replaceState: true
		});
	}

	async focusWorkSurface() {
		await tick();
		this.#workSurface?.scrollTo({ top: 0 });
		this.#workSurface?.focus({ preventScroll: true });
	}

	installWindowCloseHandler() {
		if (this.app.demo) return () => {};

		const currentWindow = getCurrentWindow();
		let disposed = false;
		let closeInProgress = false;
		let unlistenClose: (() => void) | null = null;

		const register = async () => {
			try {
				const unlisten = await currentWindow.onCloseRequested(async (event) => {
					event.preventDefault();
					if (closeInProgress) return;
					closeInProgress = true;

					try {
						await this.flushSettings();
					} catch (error) {
						this.app.notify('Close cancelled', errorMessage(error), 'error');
						closeInProgress = false;
						return;
					}

					const keepRunning = this.app.settings?.close_to_tray ?? true;
					unlistenClose?.();
					unlistenClose = null;
					try {
						// The Rust window handler is authoritative after the settings flush.
						await currentWindow.close();
					} catch (error) {
						this.app.notify('Window close failed', errorMessage(error), 'error');
						if (!disposed) void register();
						closeInProgress = false;
						return;
					}

					closeInProgress = false;
					if (keepRunning && !disposed) void register();
				});
				if (disposed) unlisten();
				else unlistenClose = unlisten;
			} catch (error) {
				if (!disposed) {
					this.app.notify('Close protection unavailable', errorMessage(error), 'error');
				}
			}
		};

		void register();
		return () => {
			disposed = true;
			unlistenClose?.();
			unlistenClose = null;
		};
	}

	async minimizeWindow() {
		try {
			await getCurrentWindow().minimize();
		} catch (error) {
			this.app.notify('Could not minimize the window', errorMessage(error), 'error');
		}
	}

	async toggleMaximizeWindow() {
		try {
			await getCurrentWindow().toggleMaximize();
		} catch (error) {
			this.app.notify('Could not resize the window', errorMessage(error), 'error');
		}
	}

	async requestWindowClose() {
		try {
			await getCurrentWindow().close();
		} catch (error) {
			this.app.notify('Could not close the window', errorMessage(error), 'error');
		}
	}

	#navigateTo(target: URL, options: NavigationOptions) {
		const destination = new SvelteURL(target);
		const navigation = this.#navigationQueue.then(async () => {
			if (navigationKey(destination) === navigationKey(page.url)) return;
			await this.#performNavigation(destination, options);
		});
		this.#navigationQueue = navigation.catch(() => {});
		return navigation;
	}

	#resumePopstate(target: URL, delta: number) {
		const destination = new SvelteURL(target);
		const navigation = this.#navigationQueue.then(async () => {
			try {
				await this.flushSettings();
			} catch {
				return;
			}
			await this.#waitForCancelledPopstate();
			this.#approvedNavigation = navigationKey(destination);
			history.go(delta);
		});
		this.#navigationQueue = navigation.catch(() => {});
		return navigation;
	}

	#waitForCancelledPopstate() {
		if (navigationKey(new SvelteURL(location.href)) === navigationKey(page.url)) {
			return Promise.resolve();
		}
		return new Promise<void>((resolveWait) => {
			const finish = () => {
				window.removeEventListener('popstate', finish);
				clearTimeout(timeout);
				resolveWait();
			};
			const timeout = window.setTimeout(finish, 1_000);
			window.addEventListener('popstate', finish, { once: true });
		});
	}

	async #performNavigation(target: URL, options: NavigationOptions) {
		if (options.flushSettings) {
			try {
				await this.flushSettings();
			} catch {
				return;
			}
		}

		const destination = pageHref(pageIdFromPathname(target.pathname), target);
		const targetKey = navigationKey(new SvelteURL(resolve(destination), page.url));
		this.#approvedNavigation = targetKey;
		try {
			await goto(resolve(destination), { replaceState: options.replaceState });
		} finally {
			if (this.#approvedNavigation === targetKey) this.#approvedNavigation = null;
		}
	}
}

export function environmentLabel(mode: ServiceMode) {
	if (mode === 'local') return 'Local';
	if (mode === 'open_router') return 'OpenRouter';
	return 'Shadoword API';
}
