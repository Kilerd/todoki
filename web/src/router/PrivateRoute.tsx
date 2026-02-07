import { ReactNode } from "react";

type Props = {
  children: ReactNode;
};

function PrivateRoute({ children }: Props) {
  return <>{children}</>;
}

export default PrivateRoute;
