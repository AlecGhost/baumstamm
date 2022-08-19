<script setup lang="ts">
import { GridItemType, GridItemWrapper } from '../types/GridItemWrapper'
import PersonCard from './PersonCard.vue';
import Person from '../types/Person';
import Connector from './Connector.vue';
import ConnectorType from '../types/ConnectorType';

const props = defineProps<{
  gridItem: GridItemWrapper
}>();

const isPerson = props.gridItem.itemType === GridItemType.Person
const isConnector = props.gridItem.itemType === GridItemType.Connector
const isEmpty = props.gridItem.itemType === GridItemType.Empty

let person: Person
let connectorType: ConnectorType
if (isPerson) {
  person = props.gridItem.item as Person
}
if (isConnector) {
  connectorType = props.gridItem.item as ConnectorType
}
</script>

<template>
  <PersonCard v-if="isPerson" :person="person" />
  <Connector v-if="isConnector" :connectorType="connectorType" />
  <div v-if="isEmpty" />
</template>
