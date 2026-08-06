import type { PageId } from '$lib/types';

export type PageRouteDescriptor = {
	id: PageId;
	path: string;
	label: string;
};

export type SettingsPageId = Extract<
	PageId,
	'settings' | 'capture' | 'transcription' | 'output' | 'application'
>;

export const PAGE_ROUTES = [
	{ id: 'transcribe', path: '/', label: 'Transcribe' },
	{ id: 'models', path: '/models', label: 'Models' },
	{ id: 'history', path: '/history', label: 'History' },
	{ id: 'settings', path: '/settings', label: 'Execution' },
	{ id: 'capture', path: '/capture', label: 'Capture' },
	{ id: 'transcription', path: '/transcription', label: 'Transcription' },
	{ id: 'output', path: '/output', label: 'Output' },
	{ id: 'application', path: '/application', label: 'Application' },
	{ id: 'about', path: '/about', label: 'About' }
] as const satisfies ReadonlyArray<PageRouteDescriptor>;

export type PagePath = (typeof PAGE_ROUTES)[number]['path'];
export type PageHref = PagePath | `${PagePath}?${string}`;

function normalizedPathname(pathname: string) {
	return pathname.length > 1 ? pathname.replace(/\/+$/, '') : pathname;
}

export function pageRoute(page: PageId) {
	return PAGE_ROUTES.find((route) => route.id === page) ?? PAGE_ROUTES[0];
}

export function pageIdFromPathname(pathname: string) {
	const normalized = normalizedPathname(pathname);
	return PAGE_ROUTES.find((route) => route.path === normalized)?.id ?? 'transcribe';
}

export function pageIdFromLegacyValue(value: string | null) {
	return PAGE_ROUTES.find((route) => route.id === value)?.id ?? 'transcribe';
}

export function isSettingsPage(page: PageId): page is SettingsPageId {
	return (
		page === 'settings' ||
		page === 'capture' ||
		page === 'transcription' ||
		page === 'output' ||
		page === 'application'
	);
}

export function settingsPageFromPathname(pathname: string) {
	const page = pageIdFromPathname(pathname);
	return isSettingsPage(page) ? page : 'settings';
}

export function pageUrl(page: PageId, source: URL) {
	const target = new URL(source);
	target.pathname = pageRoute(page).path;
	target.searchParams.delete('page');
	target.hash = '';
	return target;
}

export function pageHref(page: PageId, source: URL): PageHref {
	const searchParams = new URLSearchParams(source.search);
	searchParams.delete('page');
	const query = searchParams.toString();
	const pathname = pageRoute(page).path;
	return query ? (`${pathname}?${query}` as `${PagePath}?${string}`) : pathname;
}

export function navigationKey(url: URL) {
	return `${normalizedPathname(url.pathname)}${url.search}${url.hash}`;
}
