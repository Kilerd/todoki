import {ReactNode} from 'react'
import {Navigate} from 'react-router-dom'
import useUserSession from "../hooks/useUserSession";


type Props = {
    children: ReactNode,
    to?: string
}

function PublicRoute(props: Props) {
    const {children} = props
    const {isAuthenticated} = useUserSession()
    return isAuthenticated ? <Navigate to={props.to ?? "/"}/> : <>{children}</>
}

export default PublicRoute
