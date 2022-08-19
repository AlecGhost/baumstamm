class Person {
  firstName: string
  lastName: string | null
  dateOfBirth: string | null
  dateOfDeath: string | null

  constructor(firstName: string, lastName: string | null, dateOfBirth: string | null, dateOfDeath: string | null) {
    this.firstName = firstName
    this.lastName = lastName
    this.dateOfBirth = dateOfBirth
    this.dateOfDeath = dateOfDeath
  } 
}

export default Person
