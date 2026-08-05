import { errorMessage } from '$lib/display';
import type { DesktopStateContext } from './contracts';

export function setAppError(app: DesktopStateContext, error: unknown, context: string) {
	app.error = `${context}: ${errorMessage(error)}`;
	app.errorRetry = null;
}

export function failOverview(app: DesktopStateContext, error: unknown, context: string) {
	app.activity = 'offline';
	setAppError(app, error, context);
	app.errorRetry = 'overview';
	app.statusMessage = 'Runtime unavailable';
}

export function sentenceCase(value: string) {
	return value.replaceAll('_', ' ').replace(/^./, (character) => character.toUpperCase());
}

export function delay(milliseconds: number) {
	return new Promise<void>((resolve) => globalThis.setTimeout(resolve, milliseconds));
}
