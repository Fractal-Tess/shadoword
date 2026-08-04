import type { HistoryRecord, ModelRecord } from '$lib/types';

export const demoModels: ModelRecord[] = [
	{
		id: 'turbo',
		name: 'Large v3 Turbo',
		description: 'Fast, high-accuracy multilingual transcription for daily use.',
		size: '1.62 GiB',
		installed: true,
		selected: true,
		recommended: true
	},
	{
		id: 'large-v3',
		name: 'Large v3',
		description: 'Highest catalog accuracy when latency and memory are secondary.',
		size: '2.88 GiB',
		installed: true,
		selected: false
	},
	{
		id: 'medium-en',
		name: 'Medium English',
		description: 'English-only model with a smaller memory footprint.',
		size: '1.42 GiB',
		installed: false,
		selected: false
	},
	{
		id: 'small-en',
		name: 'Small English',
		description: 'Compact model for lower-power systems and quick drafts.',
		size: '466 MiB',
		installed: false,
		selected: false
	}
];

export const demoHistory: HistoryRecord[] = [
	{
		id: 'history-1',
		timestamp: 'Today, 09:42',
		engine: 'Remote · Large v3 Turbo',
		duration: '8.4s',
		latency: '612ms',
		segments: 3,
		text: 'Keep the API on the workstation and let the laptop remain a lightweight capture client while traveling.'
	},
	{
		id: 'history-2',
		timestamp: 'Today, 09:18',
		engine: 'Local · Large v3 Turbo',
		duration: '4.1s',
		latency: '438ms',
		segments: 1,
		text: 'The pause should close the segment without dropping the short word before it.'
	},
	{
		id: 'history-3',
		timestamp: 'Yesterday, 18:06',
		engine: 'Remote · Large v3 Turbo',
		duration: '12.7s',
		latency: '701ms',
		segments: 4,
		text: 'Archive the accepted audio beside its response metadata so we can compare the microphone input with the final transcript.'
	}
];
