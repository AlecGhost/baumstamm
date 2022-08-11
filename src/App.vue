<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import PersonCard from './components/PersonCard.vue'
import { ref } from 'vue'
import Person from './types/Person';

const person = ref<Person>({ firstName: "John", lastName: "Dow", dateOfBirth: "01.02.1234", dateOfDeath: null })
const rows = ref(4)
const columns = ref(5)

function zoomIn() {
  columns.value--
  rows.value--
}

function zoomOut() {
  columns.value++
  rows.value--
}

</script>

<template>
  <button @click="zoomOut">+</button>
  <button @click="zoomIn">-</button>
  <section class="grid-container">
    <div class="grid-item" v-for="n in rows * columns">{{ n }}</div>
  </section>
  <!-- <PersonCard :person="person"/> -->
</template>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: repeat(v-bind(columns), 1fr);
  height: 90vh;
}

.grid-item {
  background-color: green;
  border-style: solid;
  border-color: blue;
}
</style>
