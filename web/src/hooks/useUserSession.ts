import { useContext } from 'react'
import AuthContext from "../providers/AuthContext";


function useUserSession () {
  const data = useContext(AuthContext)

  return data
}

export default useUserSession
