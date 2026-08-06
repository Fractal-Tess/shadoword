import type { PageId } from '$lib/types';

export type SettingsSection = Extract<
	PageId,
	'settings' | 'capture' | 'transcription' | 'output' | 'application'
>;

export const PCM_FORMAT_OPTIONS = [
	{ value: 's16le', label: '16-bit integer', detail: 'Half bandwidth' },
	{ value: 'f32le', label: '32-bit float', detail: 'Capture native' }
];

export const SHORTCUT_MODE_OPTIONS = [
	{ value: 'push_to_talk', label: 'Push to talk', detail: 'Hold while speaking' },
	{ value: 'toggle', label: 'Toggle', detail: 'Press to start and stop' }
];

export const PASTE_METHOD_OPTIONS = [
	{ value: 'none', label: 'Disabled' },
	{ value: 'direct', label: 'Type directly' },
	{ value: 'ctrl_v', label: 'Paste with Ctrl+V' },
	{ value: 'ctrl_shift_v', label: 'Paste with Ctrl+Shift+V' },
	{ value: 'shift_insert', label: 'Paste with Shift+Insert' }
];

export const TRANSCRIPT_BOUNDARY_OPTIONS = [
	{ value: 'none', label: 'None', detail: 'No added spacing' },
	{ value: 'space', label: 'Space', detail: 'One space' },
	{ value: 'newline', label: 'New line', detail: 'Move to the next line' },
	{ value: 'blank_line', label: 'Blank line', detail: 'Leave one empty line' }
];

const PAGE_COPY = {
	capture: {
		title: 'Capture at the source.',
		description: 'Choose the microphone and global shortcut used for every execution path.'
	},
	transcription: {
		title: 'Shape transcription.',
		description: 'Control language constraints and delivery for the current execution target.'
	},
	output: {
		title: 'Deliver the text.',
		description: 'Choose how completed transcripts move into the active application.'
	},
	application: {
		title: 'Set window behavior.',
		description: 'Control how Shadoword behaves when its window closes.'
	}
} as const;

export function settingsPageCopy(
	section: SettingsSection,
	mode: 'local' | 'remote' | 'open_router'
) {
	if (section !== 'settings') return PAGE_COPY[section];
	if (mode === 'local') {
		return {
			title: 'Local runtime settings.',
			description: 'Review the Whisper runtime selected by the global execution target.'
		};
	}
	if (mode === 'open_router') {
		return {
			title: 'OpenRouter settings.',
			description: 'Configure managed batch transcription for the current execution target.'
		};
	}
	return {
		title: 'Shadoword API settings.',
		description: 'Configure the self-hosted API selected by the global execution target.'
	};
}
