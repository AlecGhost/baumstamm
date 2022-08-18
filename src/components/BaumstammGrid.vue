<script setup lang="ts">
import { ref } from 'vue'
import Person from '../types/Person'
import PersonCard from 'PersonCard.vue'
import ConnectorType from '../types/ConnectorType'
import Connector from './Connector.vue'

const props = defineProps<{
  rows: number,
  columns: number
}>();
const person = ref<Person>({ firstName: "John", lastName: "Dow", dateOfBirth: "01.02.1234", dateOfDeath: null })
const connectorType = ref<ConnectorType>(ConnectorType.Straight)

</script>

<template>
  <section class="grid-container">
    <div class="grid-item" v-for="n in rows * columns">
      <Connector :connectorType="connectorType"></Connector>
    </div>
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
