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

  private constructor(itemType: GridItemType, item: Person | ConnectorType | null) {
    this.itemType = itemType
    this.item = item
  }

  public static deserialize(gridItem: any): GridItemWrapper {
    if (gridItem === "None") {
      return new GridItemWrapper(GridItemType.Empty, null)
    }
    if (gridItem["Connector"]) {
      const connector = gridItem["Connector"]
      let connectorType: ConnectorType
      switch (connector) {
        case "T":
          connectorType = ConnectorType.T
          break;
        case "Straight":
          connectorType = ConnectorType.Straight
          break;
        case "LeftCorner":
          connectorType = ConnectorType.LeftCorner
          break;
        case "RightCorner":
          connectorType = ConnectorType.RightCorner
          break;
        default:
         throw new Error("Deserialization failed")
      }
      return new GridItemWrapper(GridItemType.Connector, connectorType)
    }
    if (gridItem["Person"]) {
      const person = gridItem["Person"]
      return new GridItemWrapper(GridItemType.Person, new Person(person["first_name"], person["last_name"], person["date_of_birth"], person["date_of_death"]))
    }
    throw new Error("Deserialization failed")
  }
}

export { GridItemWrapper, GridItemType} 
