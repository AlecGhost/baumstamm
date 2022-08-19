import Person from './Person'
import ConnectorType from './ConnectorType'

enum GridItemType {
  Person,
  Connector,
  Empty
}

class GridItemWrapper {
  itemType: GridItemType
  item: Person | ConnectorType | null

  constructor(itemType: GridItemType, item: Person | ConnectorType | null) {
    this.itemType = itemType
    this.item = item
  }
}

export { GridItemWrapper, GridItemType} 
