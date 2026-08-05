<script lang="ts">
	import * as Select from '$lib/components/ui/select';

	type Option = {
		value: string;
		label: string;
		detail?: string;
		disabled?: boolean;
	};

	let {
		id,
		value = $bindable(''),
		options,
		disabled = false,
		ariaLabel,
		ariaInvalid,
		ariaDescribedBy,
		menuLabel,
		onValueChange
	}: {
		id?: string;
		value?: string;
		options: readonly Option[];
		disabled?: boolean;
		ariaLabel?: string;
		ariaInvalid?: boolean;
		ariaDescribedBy?: string;
		menuLabel?: string;
		onValueChange?: (value: string) => void;
	} = $props();

	let selected = $derived(options.find((option) => option.value === value) ?? null);
</script>

<Select.Root type="single" bind:value {disabled} {onValueChange}>
	<Select.Trigger
		{id}
		class="instrument-select-trigger"
		aria-label={ariaLabel}
		aria-invalid={ariaInvalid}
		aria-describedby={ariaDescribedBy}
	>
		<span class="instrument-select-value">
			<strong>{selected?.label ?? value}</strong>
			{#if selected?.detail}<small>{selected.detail}</small>{/if}
		</span>
	</Select.Trigger>
	<Select.Content class="instrument-select-content" sideOffset={6}>
		{#if menuLabel}<Select.Label>{menuLabel}</Select.Label>{/if}
		{#each options as option (option.value)}
			<Select.Item
				value={option.value}
				label={option.label}
				class="instrument-select-item"
				disabled={option.disabled}
			>
				<span class="instrument-select-option">
					<strong>{option.label}</strong>
					{#if option.detail}<small>{option.detail}</small>{/if}
				</span>
			</Select.Item>
		{/each}
	</Select.Content>
</Select.Root>

<style>
	.instrument-select-value,
	.instrument-select-option {
		display: grid;
		min-width: 0;
		gap: 0.08rem;
		text-align: left;
	}

	.instrument-select-value {
		flex: 1;
	}

	.instrument-select-value strong,
	.instrument-select-option strong {
		overflow: hidden;
		color: var(--ink);
		font-size: 0.68rem;
		font-weight: 590;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.instrument-select-value small,
	.instrument-select-option small {
		overflow: hidden;
		color: var(--ink-muted);
		font-size: 0.56rem;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
