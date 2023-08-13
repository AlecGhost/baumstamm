<script lang="ts">
	import { persons, selected, update, updateSelected } from '$lib/store';
	import {
		addNewRelationship,
		addParent,
		addRelationshipWithPartner,
		type PersonId,
		type Relationship
	} from '../../bindings';

	export let ownRelationships: Relationship[];
	export let pid: PersonId;

	async function newPartner() {
		let rid = await addNewRelationship(pid);
		let [partner, _] = await addParent(rid);
		await update();
		updateSelected(partner);
	}

	let existingPartner: PersonId;
	async function newRelationshipWithPartner() {
		let partner = await addRelationshipWithPartner(pid, existingPartner);
		await update();
		updateSelected(partner);
	}
</script>

<section class="p-4">
	<span class="label">Partners:</span>
	<div class="table-container m-1">
		<table class="table table-hover">
			<tbody>
				{#each ownRelationships
					.flatMap((rel) => rel.parents)
					.filter((parent) => parent !== null && parent !== pid)
					.map((parent) => $persons.find((person) => person.id == parent)) as partner}
					<tr on:click={() => $selected = partner ?? null} class="cursor-pointer">
						<td class="table-cell-fit">{partner?.name()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
	<button on:click={newPartner} type="button" class="btn variant-filled m-1">Add new Partner</button
	>
	<button on:click={newRelationshipWithPartner} type="button" class="btn variant-filled m-1"
		>Add existing Partner</button
	>
	<select bind:value={existingPartner} class="select m-1">
		{#each $persons.filter((p) => p.id !== pid && !ownRelationships.some( (rel) => rel.parents.includes(p.id) )) as partner}
			<option value={partner.id}>{partner.name()}</option>
		{/each}
	</select>
</section>
