<script setup lang="ts">
import Person from '../types/Person'

defineProps<{
  person: Person
}>()
</script>

<template>
  <h1>{{ person.firstName }} {{ person.lastName }}</h1>
  <p v-if="person.dateOfDeath">{{ person.dateOfBirth }} - {{ person.dateOfDeath }}</p>
  <p v-else>* {{ person.dateOfBirth }}</p>
</template>
