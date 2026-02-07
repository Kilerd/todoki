import {ReactNode, useEffect, useState} from 'react'
import {useLocation, useNavigate} from 'react-router-dom'

import {api} from '../services/api'
import {setAuthorizationHeader} from '../services/interceptors'
import {createTokenCookies, getToken, removeTokenCookies} from '../utils/tokenCookies'
import AuthContext, {SignInCredentials, User} from "./AuthContext";

type Props = {
    children: ReactNode
}

function AuthProvider(props: Props) {
    const {children} = props

    const [user, setUser] = useState<User | null>()
    const [loadingUserData, setLoadingUserData] = useState(true)
    const navigate = useNavigate()
    const {pathname} = useLocation()
    const token = getToken()
    const isAuthenticated = Boolean(token)
    const userData = user as User

    async function signIn(params: SignInCredentials) {
        const {email, password} = params
        const response = await api.post('/users/session', {email, password})
        const {username, token, refreshToken, permissions, roles} = response.data

        createTokenCookies(token, refreshToken)
        setUser({username, email, permissions, roles})
        setAuthorizationHeader(api.defaults, token)
    }

    function signOut(pathname: string) {
        console.log("sign out", pathname)
        removeTokenCookies()
        setUser(null)
        setLoadingUserData(false)
        navigate(pathname)
    }

    useEffect(() => {
        if (!token) signOut(pathname)
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [pathname, token])

    useEffect(() => {
        const token = getToken()

        async function getUserData() {
            setLoadingUserData(true)

            try {
                const response = await api.get('/users/me')

                if (response?.data) {
                    const {username, email, permissions, roles} = response.data
                    setUser({username, email, permissions, roles})
                }
            } catch (error) {
                signOut("/")
            }

            setLoadingUserData(false)
        }

        if (token) {
            setAuthorizationHeader(api.defaults, token)
            getUserData()
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    return (
        <AuthContext.Provider value={{
            isAuthenticated,
            user: userData,
            loadingUserData,
            signIn,
            signOut
        }}>
            {children}
        </AuthContext.Provider>
    )
}

export default AuthProvider
