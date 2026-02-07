import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useState } from "react";
import { setToken, validateToken } from "@/lib/auth";
import { Loader2 } from "lucide-react";

interface Props {
  onSuccess: () => void;
}

export default function TokenInput({ onSuccess }: Props) {
  const [token, setTokenValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!token.trim()) return;

    setIsLoading(true);
    setError("");

    const isValid = await validateToken(token.trim());

    if (isValid) {
      setToken(token.trim());
      onSuccess();
    } else {
      setError("Token 无效或服务器无法连接");
    }

    setIsLoading(false);
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <Card className="w-[400px]">
        <CardHeader>
          <CardTitle>Todoki</CardTitle>
          <CardDescription>请输入访问令牌以继续</CardDescription>
        </CardHeader>
        <form onSubmit={handleSubmit}>
          <CardContent>
            <Input
              type="password"
              placeholder="Access Token"
              value={token}
              onChange={(e) => setTokenValue(e.target.value)}
              disabled={isLoading}
            />
            {error && <p className="text-sm text-red-500 mt-2">{error}</p>}
          </CardContent>
          <CardFooter>
            <Button
              type="submit"
              className="w-full"
              disabled={isLoading || !token.trim()}
            >
              {isLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  验证中...
                </>
              ) : (
                "确认"
              )}
            </Button>
          </CardFooter>
        </form>
      </Card>
    </div>
  );
}
