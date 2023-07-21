<script lang="ts">
	import { Avatar, toastStore, type ToastSettings } from '@skeletonlabs/skeleton';
	import type { Person } from './Person';
	import { removePerson, type Relationship } from '../bindings';
	import { relationships, update } from './store';
	import { onDestroy, onMount } from 'svelte';
	import type { Unsubscriber } from 'svelte/store';
	import InfoSection from './sidebar/InfoSection.svelte';
	import ParentSection from './sidebar/ParentSection.svelte';
	import ChildSection from './sidebar/ChildSection.svelte';
	import PartnerSection from './sidebar/PartnerSection.svelte';
	import NameSection from './sidebar/NameSection.svelte';

	export let person: Person | null;

	// stores
	let unsubscribe: Unsubscriber;
	let relationshipStore: Relationship[] = [];

	onMount(() => {
		unsubscribe = relationships.subscribe((store) => (relationshipStore = store));
	});

	onDestroy(() => {
		unsubscribe();
	});

	let ownRelationships: Relationship[] = [];
	$: {
		if (person !== null) {
			ownRelationships = relationshipStore.filter((rel) => rel.parents.includes(person!.id));
		}
	}
	let parentRel: Relationship;
	$: {
		if (person !== null) {
			parentRel = relationshipStore.find((rel) => rel.children.includes(person!.id))!;
		}
	}

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
</script>

<section>
	{#if person !== null}
		<section class="flex justify-center">
			<Avatar src={person.avatar()} initials={person.initials()} width="w-60" />
		</section>
		<NameSection {person} />
		<InfoSection {person} />
		<hr />
		{#if parentRel !== undefined}
			<ParentSection {parentRel} />
		{/if}
		<hr />
		<ChildSection {ownRelationships} pid={person.id} />
		<hr />
		<PartnerSection {ownRelationships} pid={person.id} />
		<hr />
		<section class="p-4">
			<button on:click={deletePerson} class="btn variant-filled-error m-1">Delete Person</button>
		</section>
	{:else}
		<section class="flex justify-center">
			<div class="placeholder-circle animate-pulse w-60" />
		</section>
		<p class="text-center">No person selected.</p>
	{/if}
</section>

<style>
	section {
		padding: 1rem;
	}
</style>
