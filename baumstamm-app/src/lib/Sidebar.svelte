<script lang="ts">
	import { Avatar, focusTrap } from '@skeletonlabs/skeleton';
	import type { Person } from './Person';
	import {
		addParent,
		insertInfo,
		removeInfo,
		type Relationship,
		addNewRelationship,
		type RelationshipId,
		addChild,
		addRelationshipWithPartner,
		type PersonId
	} from '../bindings';
	import { update, relationships, persons } from './store';
	import { onDestroy, onMount } from 'svelte';
	import type { Unsubscriber } from 'svelte/store';

	export let person: Person | null;

	// stores
	let unsubscribe: Unsubscriber[] = [];
	let personStore: Person[] = [];
	let relationshipStore: Relationship[] = [];

	onMount(() => {
		let unsubscribePersons = persons.subscribe((store) => (personStore = store));
		let unsubscribeRels = relationships.subscribe((store) => (relationshipStore = store));
		unsubscribe.push(unsubscribePersons);
		unsubscribe.push(unsubscribeRels);
	});

	onDestroy(() => {
		unsubscribe.forEach((unsubscribe) => unsubscribe());
	});

	// info input
	let isFocused: boolean = true;
	let infoKey = '';
	let infoValue = '';

	function submitInfo() {
		if (person !== null) {
			insertInfo(person.id, infoKey, infoValue);
			update();
			person.info.set(infoKey, infoValue);
			// let svelte know, that the value updated
			person = person;
		}
		infoKey = '';
		infoValue = '';
	}

	function deleteInfo() {
		if (person !== null) {
			removeInfo(person.id, infoKey);
			update();
			person.info.delete(infoKey);
			// let svelte know, that the value updated
			person = person;
		}
		infoKey = '';
		infoValue = '';
	}

	// add nodes
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
			console.log('added parent rel', parentRel);
		}
	}

	function newParent() {
		if (person !== null) {
			addParent(parentRel.id);
			update();
		}
	}

	function calculateLabel(rel: Relationship): string {
		const partnerId = rel.parents.find((parent) => parent !== person!.id);
		if (partnerId !== null && partnerId !== undefined) {
			const partner = personStore.find((person) => person.id == partnerId)!;
			return partner.name();
		}
		return 'Unknown';
	}

	let childPartner: RelationshipId;
	function newChild() {
		addChild(childPartner);
		update();
	}

	async function newPartner() {
		if (person !== null) {
			let rid = await addNewRelationship(person.id);
			addParent(rid);
			update();
		}
	}

	let existingPartner: PersonId;
	function newRelationshipWithPartner() {
		addRelationshipWithPartner(person!.id, existingPartner);
	}
</script>

<div class="p-4">
	{#if person !== null}
		<section class="flex justify-center p-4">
			<Avatar src={person.avatar()} initials={person.initials()} width="w-60" />
		</section>
		<p class="font-bold text-center">{person.name()}</p>
		<section class="p-4">
			<div class="table-container m-1">
				<table class="table table-hover">
					<tbody>
						{#each person.info as [key, value]}
							<tr
								on:click={() => {
									infoKey = key;
									infoValue = value;
								}}
							>
								<td class="table-cell-fit">{key}</td>
								<td class="table-cell-fit">{value}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<form on:submit|preventDefault={submitInfo} use:focusTrap={isFocused}>
				<input type="text" placeholder="Key" class="input m-1" bind:value={infoKey} />
				<input type="text" placeholder="Value" class="input m-1" bind:value={infoValue} />
				<button type="submit" class="btn variant-filled-primary m-1">Add</button>
				{#if person.info.has(infoKey)}
					<button on:click={deleteInfo} type="button" class="btn variant-filled-error m-1"
						>Delete</button
					>
				{/if}
			</form>
		</section>
		<hr />
		<section class="p-4">
			<span class="label">Parents:</span>
			{#if parentRel !== undefined}
				<div class="table-container m-1">
					<table class="table table-hover">
						<tbody>
							{#each parentRel.parents
								.filter((parent) => parent !== null)
								.map((pid) => personStore.find((person) => person.id == pid)) as parent}
								<tr>
									<td class="table-cell-fit">{parent?.name()}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				{#if parentRel.parents.filter((parent) => parent !== null).length < 2}
					<button on:click={newParent} type="button" class="btn variant-filled m-1"
						>Add Parent</button
					>
				{/if}
			{/if}
		</section>
		<hr />
		<section class="p-4">
			<span class="label">Children:</span>
			{#if ownRelationships.length != 0}
				<div class="table-container m-1">
					<table class="table table-hover">
						<tbody>
							{#each ownRelationships
								.flatMap((rel) => rel.children)
								.map((pid) => personStore.find((person) => person.id == pid)) as child}
								<tr>
									<td class="table-cell-fit">{child?.name()}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				<button on:click={newChild} type="button" class="btn variant-filled m-1"
					>Add Child with</button
				>
				<select bind:value={childPartner} class="select m-1">
					{#each ownRelationships as rel}
						<option value={rel.id}>{calculateLabel(rel)}</option>
					{/each}
				</select>
			{/if}
		</section>
		<hr />
		<section class="p-4">
			<span class="label">Partners:</span>
			<div class="table-container m-1">
				<table class="table table-hover">
					<tbody>
						{#each ownRelationships
							.flatMap((rel) => rel.parents)
							.filter((parent) => parent !== null && person !== null && parent !== person.id)
							.map((pid) => personStore.find((person) => person.id == pid)) as child}
							<tr>
								<td class="table-cell-fit">{child?.name()}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<button on:click={newPartner} type="button" class="btn variant-filled m-1"
				>Add new Partner</button
			>
			<button />
			<button on:click={newRelationshipWithPartner} type="button" class="btn variant-filled m-1"
				>Add existing Partner</button
			>
			<select bind:value={existingPartner} class="select m-1">
				{#each personStore.filter((p) => person !== null && p.id !== person.id && !ownRelationships.some( (rel) => rel.parents.includes(p.id) )) as partner}
					<option value={partner.id}>{partner.name()}</option>
				{/each}
			</select>
		</section>
	{:else}
		<div class="flex justify-center p-4">
			<div class="placeholder-circle animate-pulse w-60" />
		</div>
		<p class="text-center">No person selected.</p>
	{/if}
</div>
