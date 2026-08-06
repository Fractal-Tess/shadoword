<script lang="ts">
	import SelectContent from './select-content.svelte';
	import SelectItem from './select-item.svelte';
	import SelectLabel from './select-label.svelte';
	import SelectRoot from './select-root.svelte';
	import SelectTrigger from './select-trigger.svelte';

	type SelectOption = {
		value: string;
		label: string;
		detail?: string;
		disabled?: boolean;
	};

	type SelectProps = {
		id?: string;
		value?: string;
		options: readonly SelectOption[];
		disabled?: boolean;
		class?: string;
		contentClass?: string;
		itemClass?: string;
		ariaLabel?: string;
		ariaInvalid?: boolean;
		ariaDescribedBy?: string;
		ariaBusy?: boolean;
		menuLabel?: string;
		onValueChange?: (value: string) => void;
	};

	let {
		id,
		value = $bindable(''),
		options,
		disabled = false,
		class: className,
		contentClass,
		itemClass,
		ariaLabel,
		ariaInvalid,
		ariaDescribedBy,
		ariaBusy,
		menuLabel,
		onValueChange
	}: SelectProps = $props();

	let selected = $derived(options.find((option) => option.value === value) ?? null);
</script>

<SelectRoot type="single" bind:value {disabled} {onValueChange}>
	<SelectTrigger
		{id}
		class={className}
		aria-label={ariaLabel}
		aria-invalid={ariaInvalid}
		aria-describedby={ariaDescribedBy}
		aria-busy={ariaBusy}
	>
		<span class="grid min-w-0 flex-1 gap-[0.08rem] text-left">
			<strong
				class="overflow-hidden text-[0.68rem] font-[590] text-ellipsis whitespace-nowrap text-ink"
			>
				{selected?.label ?? value}
			</strong>
			{#if selected?.detail}
				<small
					class="overflow-hidden text-[0.625rem] text-ellipsis whitespace-nowrap text-ink-muted"
				>
					{selected.detail}
				</small>
			{/if}
		</span>
	</SelectTrigger>
	<SelectContent class={contentClass} sideOffset={6}>
		{#if menuLabel}<SelectLabel>{menuLabel}</SelectLabel>{/if}
		{#each options as option (option.value)}
			<SelectItem
				value={option.value}
				label={option.label}
				disabled={option.disabled}
				class={itemClass}
			>
				<span class="grid min-w-0 gap-[0.08rem] text-left">
					<strong
						class="overflow-hidden text-[0.68rem] font-[590] text-ellipsis whitespace-nowrap text-ink"
					>
						{option.label}
					</strong>
					{#if option.detail}
						<small
							class="overflow-hidden text-[0.625rem] text-ellipsis whitespace-nowrap text-ink-muted"
						>
							{option.detail}
						</small>
					{/if}
				</span>
			</SelectItem>
		{/each}
	</SelectContent>
</SelectRoot>
