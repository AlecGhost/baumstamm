<script lang="ts">
	import { toastStore, type ToastSettings } from '@skeletonlabs/skeleton';
	import { removePerson, mergePerson, type PersonId } from '$lib/../bindings';
	import { update, persons } from '$lib/store';
	import type { Person } from '$lib/Person';

	export let person: Person;

	function deletePerson() {
		removePerson(person!.id)
			.then(update)
			.catch((err: string) => {
				const toast: ToastSettings = {
					message: err
				};
				toastStore.trigger(toast);
			});
	}

	let mergeTarget: PersonId;
	function mergeWithPerson() {
		mergePerson(person!.id, mergeTarget)
			.then(update)
			.catch((err: string) => {
				const toast: ToastSettings = {
					message: err
				};
				toastStore.trigger(toast);
			});
	}
</script>

<section class="p-4">
	{#if $persons.filter((p) => p.id !== person.id && p.name() == person.name()).length != 0}
		<button on:click={mergeWithPerson} class="btn variant-filled-warning m-1"
			>Merge with Person</button
		>
		<select bind:value={mergeTarget} class="select m-1">
			{#each $persons.filter((p) => p.id !== person.id && p.name() == person.name()) as other}
				<option value={other.id}>{other.name()}</option>
			{/each}
		</select>
	{/if}
	<button on:click={deletePerson} class="btn variant-filled-error m-1">Delete Person</button>
</section>
