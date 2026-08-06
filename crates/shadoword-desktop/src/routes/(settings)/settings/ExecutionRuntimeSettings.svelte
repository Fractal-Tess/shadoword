<script lang="ts">
	import { Cpu, HardDrive, MonitorCog } from '@lucide/svelte';
	import { Badge } from '$lib/components/ui/badge';
	import ExecutionPool from './execution-pool/ExecutionPool.svelte';
	import SettingsPanel from '../SettingsPanel.svelte';
	import SettingsRow from '../SettingsRow.svelte';
	import { formatBytes } from '$lib/display';
	import { getSettingsContext } from '../_state/context.svelte';

	const settings = getSettingsContext();
	let runtime = $derived(settings.activeRuntime);
	let status = $derived(settings.app.overview?.status ?? null);
	let devices = $derived(status?.available_gpu_devices ?? []);
	let compiledBackends = $derived(status?.compiled_whisper_backends ?? ['cpu']);
	const allBackends = ['cpu', 'cuda', 'vulkan'] as const;
</script>

<SettingsPanel class="mb-4">
	<SettingsRow>
		<div>
			<span class="inline-flex items-center gap-[0.4rem] text-xs font-[570] text-ink">
				<Cpu size={14} aria-hidden="true" /> Build support
			</span>
			<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
				Backends compiled into this Shadoword runtime.
			</p>
		</div>
		<div
			class="flex flex-wrap justify-end gap-[0.4rem] max-[800px]:justify-start"
			aria-label="Compiled Whisper backends"
		>
			{#each allBackends as backend (backend)}
				<Badge
					variant="outline"
					class={compiledBackends.includes(backend) ? undefined : 'opacity-[0.45]'}
				>
					{backend.toUpperCase()} · {compiledBackends.includes(backend) ? 'built' : 'not built'}
				</Badge>
			{/each}
		</div>
	</SettingsRow>
	<SettingsRow>
		<div>
			<span class="inline-flex items-center gap-[0.4rem] text-xs font-[570] text-ink">
				<MonitorCog size={14} aria-hidden="true" /> Detected hardware
			</span>
			<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
				Devices currently available to the selected runtime.
			</p>
		</div>
		<div
			class="grid justify-items-end gap-1 text-right font-mono text-[0.6875rem] text-ink-muted max-[800px]:justify-items-start max-[800px]:text-left"
		>
			{#if devices.length > 0}
				{#each devices as device (device.id)}
					<span>GPU {device.id} · {device.name} · {formatBytes(device.free_vram)} free</span>
				{/each}
			{:else}
				<span>CPU execution available · no compatible GPU detected</span>
			{/if}
		</div>
	</SettingsRow>
	<SettingsRow class="grid-cols-1 gap-[0.7rem]">
		<div>
			<span class="inline-flex items-center gap-[0.4rem] text-xs font-[570] text-ink">
				<HardDrive size={14} aria-hidden="true" /> Active model path
			</span>
			<p class="mt-[0.2rem] text-[0.6875rem] leading-[1.45] text-ink-muted">
				The model file used by this execution target.
			</p>
		</div>
		<code
			class="block min-w-0 border border-line bg-night px-3 py-[0.65rem] text-[0.6875rem] leading-[1.45] [overflow-wrap:anywhere] text-ink-dim"
		>
			{runtime?.model_path || 'No model selected'}
		</code>
	</SettingsRow>
</SettingsPanel>

{#if runtime}
	{#key `${settings.mode}-${runtime.generation ?? runtime.model_path}`}
		<ExecutionPool app={settings.app} {runtime} gpuDevices={devices} />
	{/key}
{/if}
