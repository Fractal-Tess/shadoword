<script lang="ts">
	import { Check, CloudDownload, HardDrive, Trash2, X } from '@lucide/svelte';
	import { Dialog } from 'bits-ui';
	import type { ModelInfoDto } from '$lib/bindings';
	import { Badge } from '$lib/components/ui/badge';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import { Progress } from '$lib/components/ui/progress';
	import { downloadPercent, formatBytes } from '$lib/display';
	import { cn } from '$lib/utils';
	import { getModelsContext } from './state.svelte';

	let { model }: { model: ModelInfoDto } = $props();
	const context = getModelsContext();
	let download = $derived(context.app.downloads[model.id]);
	let deleteDialogOpen = $state(false);

	function deleteModel() {
		deleteDialogOpen = false;
		void context.app.deleteModel(model.id);
	}
</script>

<article
	class={cn(
		'grid min-h-[6.4rem] grid-cols-[2.5rem_minmax(0,1fr)_auto] items-center gap-[0.9rem] border-t border-line p-4 transition-colors duration-[140ms] ease-[ease] first:border-t-0 hover:bg-raised max-[720px]:grid-cols-[2.5rem_minmax(0,1fr)]',
		context.selectedId === model.id && 'bg-raised shadow-[inset_2px_0_0_var(--scarlet)]',
		context.controlsLocked && 'opacity-[0.58]'
	)}
>
	<div
		class={cn(
			'grid size-[2.35rem] place-items-center border border-line text-ink-muted',
			context.selectedId === model.id && 'border-scarlet text-scarlet-lamp'
		)}
		aria-hidden="true"
	>
		{#if model.installed}<HardDrive size={18} />{:else}<CloudDownload size={18} />{/if}
	</div>
	<div class="min-w-0">
		<div class="flex items-center gap-[0.45rem]">
			<h3
				class="m-0 font-display text-[1.125rem] leading-none font-normal tracking-[0.035em] text-ink uppercase"
			>
				{model.name}
			</h3>
			{#if model.recommended}<Badge variant="outline">Recommended</Badge>{/if}
			{#if context.selectedId === model.id}
				<Badge class="border-scarlet bg-transparent text-scarlet-lamp">
					<Check size={11} />Selected
				</Badge>
			{/if}
		</div>
		<p class="my-[0.35rem] text-[0.72rem] leading-[1.45] text-ink-dim">
			{model.description}
		</p>
		<div class="flex items-center gap-[0.45rem] font-mono text-[0.6875rem] text-ink-muted">
			<span>{formatBytes(model.size_bytes)}</span>
			<span class="before:mr-[0.45rem] before:content-['·']">
				{model.installed
					? 'Installed'
					: context.mode === 'remote'
						? 'Not on API'
						: 'Not on this machine'}
			</span>
		</div>
		{#if download && (download.state === 'queued' || download.state === 'running')}
			<div
				class="mt-[0.65rem] flex max-w-56 items-center gap-[0.45rem] font-mono text-[0.6875rem] text-ink-muted"
			>
				<Progress class="h-1" value={downloadPercent(download)} />
				<span>{downloadPercent(download)}% · {download.state}</span>
			</div>
		{/if}
	</div>
	<div
		class="flex w-[11.25rem] min-w-[11.25rem] flex-col items-stretch gap-[0.35rem] pl-4 max-[720px]:col-start-2 max-[720px]:w-[min(100%,11.25rem)] max-[720px]:min-w-0 max-[720px]:p-0"
	>
		{#if context.app.downloadWatching[model.id]}
			<Button
				variant="outline"
				size="sm"
				class="w-full justify-center"
				onclick={() => context.app.stopWatchingDownload(model.id)}
			>
				<X size={13} />Stop watching
			</Button>
		{:else if model.installed}
			<Button
				variant={context.selectedId === model.id ? 'secondary' : 'default'}
				size="sm"
				class={cn(
					'w-full justify-center',
					context.selectedId === model.id &&
						'border-line bg-raised text-ink-muted disabled:opacity-80'
				)}
				disabled={context.selectedId === model.id || context.controlsLocked}
				onclick={() => context.app.selectModel(model.id)}
			>
				{context.selectedId === model.id
					? 'In use'
					: context.mode === 'remote'
						? 'Select on API'
						: 'Select locally'}
			</Button>
			{#if context.selectedId !== model.id}
				<Dialog.Root bind:open={deleteDialogOpen}>
					<Dialog.Trigger
						class={cn(
							buttonVariants({ variant: 'destructive', size: 'sm' }),
							'w-full justify-center border-scarlet/65 bg-scarlet/15 text-scarlet-lamp hover:border-scarlet hover:bg-scarlet hover:text-on-scarlet dark:bg-scarlet/15 dark:hover:bg-scarlet'
						)}
						disabled={context.controlsLocked}
						aria-label={`Delete ${model.name}`}
					>
						<Trash2 size={13} />Delete
					</Dialog.Trigger>
					<Dialog.Portal>
						<Dialog.Overlay
							class="fixed inset-0 isolate z-50 bg-black/10 duration-100 supports-backdrop-filter:backdrop-blur-xs motion-reduce:animate-none data-open:animate-in data-open:fade-in-0 data-closed:animate-out data-closed:fade-out-0"
						/>
						<Dialog.Content
							class="fixed top-1/2 left-1/2 z-50 grid w-full max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 gap-4 border border-line-strong bg-plate p-4 text-sm text-ink shadow-[0_1rem_3rem_rgb(0_0_0/0.55)] duration-100 outline-none motion-reduce:animate-none sm:max-w-sm data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95"
						>
							<div class="flex flex-col gap-2">
								<Dialog.Title class="font-display text-xl tracking-[0.035em] uppercase">
									Delete {model.name}?
								</Dialog.Title>
								<Dialog.Description class="text-[0.75rem] leading-[1.55] text-ink-dim">
									This removes the model weights from {context.mode === 'remote'
										? 'the Shadoword API host'
										: 'this machine'}. Download them again to restore the model.
								</Dialog.Description>
							</div>
							<div
								class="-mx-4 -mb-4 flex flex-col-reverse gap-2 border-t bg-raised/50 p-4 sm:flex-row sm:justify-end"
							>
								<Dialog.Close class={buttonVariants({ variant: 'outline', size: 'sm' })}>
									Keep model
								</Dialog.Close>
								<Button
									variant="destructive"
									size="sm"
									class="border border-scarlet bg-scarlet text-on-scarlet hover:bg-scarlet-deep"
									onclick={deleteModel}
								>
									<Trash2 size={13} />Delete model
								</Button>
							</div>
						</Dialog.Content>
					</Dialog.Portal>
				</Dialog.Root>
			{/if}
		{:else}
			<Button
				size="sm"
				class="w-full justify-center"
				disabled={context.controlsLocked}
				onclick={() => context.app.startDownload(model.id)}
			>
				<CloudDownload size={14} />
				{context.mode === 'remote' ? 'Download to API' : 'Download'}
			</Button>
		{/if}
	</div>
</article>
