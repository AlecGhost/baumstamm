<script lang="ts">
	import type { Person } from '$lib/Person';
	import { update } from '$lib/store';
	import { focusTrap } from '@skeletonlabs/skeleton';
	import { insertInfo } from '$lib/api';

	export let person: Person;

	let isFocused = false;

	let firstName = person.firstName ?? '';
	let lastName = person.lastName ?? '';
	let showForm = !person.firstName || !person.lastName;

	// reevaluate if person changes
	let pid = person.id;
	$: if (person.id !== pid) {
		firstName = person.firstName ?? '';
		lastName = person.lastName ?? '';
		showForm = !person.firstName || !person.lastName;
		pid = person.id;
	}

	function submitName() {
		insertInfo(person.id, '@firstName', firstName);
		insertInfo(person.id, '@lastName', lastName);
		update();
		person.firstName = firstName;
		person.lastName = lastName;
		showForm = false;
	}
</script>

<section class="p-4">
	{#if showForm}
		<form on:submit|preventDefault={submitName} use:focusTrap={isFocused}>
			<input type="text" placeholder="First Name" class="input m-1" bind:value={firstName} />
			<input type="text" placeholder="Last Name" class="input m-1" bind:value={lastName} />
			<button type="submit" class="btn variant-filled-primary m-1">Add</button>
		</form>
	{:else}
		<p on:click={() => (showForm = true)} class="font-bold text-center">{person.name()}</p>
	{/if}
</section>
{#if showForm}
	<hr />
{/if}
