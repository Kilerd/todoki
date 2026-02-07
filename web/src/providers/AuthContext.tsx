import { AxiosError } from 'axios'
import { createContext } from 'react'

export type User = {
  username: string,
  email: string
  permissions: string[]
  roles: string[]
}

export type SignInCredentials = {
  email: string
  password: string
}

export type AuthContextData = {
  signIn: (credentials: SignInCredentials) => Promise<void | AxiosError>
  signOut: (pathname: string) => void
  user?: User
  isAuthenticated: boolean
  loadingUserData: boolean
}

const AuthContext = createContext({} as AuthContextData)

export default AuthContext
