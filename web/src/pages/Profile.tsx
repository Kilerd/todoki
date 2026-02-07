import { Avatar, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { useToast } from "@/hooks/use-toast";
import { supported } from "@github/webauthn-json/browser-ponyfill";
import { base64urlToBuffer, bufferToBase64url } from "@github/webauthn-json/extended";
import NavBar from "../components/NavBar";
import useUserSession from "../hooks/useUserSession";
import { api, dataExport } from "../services/api";
import basicCV from "../static/bendy-recruiter-chooses-the-best-candidate-cv.png";
import dataPrivacy from "../static/bendy-user-authentication-in-mobile-application.png";

export default function Profile() {
    const {user} = useUserSession();
    const { toast } = useToast();
    // const {data: passkeyOptions} = useSWR<any>("/users/passkey/options", fetcher);

    const onDataExport = async () => {
        const res = await dataExport();
        const data = JSON.stringify(res, null, 4);
        const url = window.URL.createObjectURL(new Blob([data]));
        const link = document.createElement("a");
        link.href = url;
        link.setAttribute("download", `toodoo_data.json`);

        // Append to html link element page
        document.body.appendChild(link);

        // Start download
        link.click();

        // Clean up and remove the link
        // @ts-ignore
        link.parentNode.removeChild(link);
        toast({
            title: "导出成功",
            description: "数据已成功导出，并下载成功",
        });
    };

    const onCreatePasskey = async () => {
        const json = await (await api.get("/users/passkey/registry/options")).data;
        console.log("registry options", json);

        // const options = parseCreationOptionsFromJSON(json);

        json.publicKey.challenge = base64urlToBuffer(json.publicKey.challenge);
        json.publicKey.user.id = base64urlToBuffer(json.publicKey.user.id);
        json.publicKey.excludeCredentials?.forEach(function (listItem: any) {
            listItem.id = base64urlToBuffer(listItem.id);
        });

        let credential = await navigator.credentials.create({
            publicKey: json.publicKey,
        });

        const resJson = {
            // @ts-ignore
            id: credential.id,
            // @ts-ignore
            rawId: bufferToBase64url(credential.rawId),
            // @ts-ignore
            type: credential.type,
            response: {
                // @ts-ignore
                attestationObject: bufferToBase64url(credential.response.attestationObject),
                // @ts-ignore
                clientDataJSON: bufferToBase64url(credential.response.clientDataJSON),
            },
        };
        console.log("data", resJson);
        await api.post("/users/passkey/registry", resJson);
    };

    const onPasskeyAuthenticate = async () => {
        if (supported()) {
            const data = (await api.get("/users/passkey/options")).data;
            const {counter, challenge} = data;
            challenge.publicKey.challenge = base64urlToBuffer(challenge.publicKey.challenge);
            challenge.publicKey.allowCredentials?.forEach(function (listItem: any) {
                listItem.id = base64urlToBuffer(listItem.id)
            });

            const fetchedCredential = await  navigator.credentials.get({
                publicKey: challenge.publicKey
            });

            if (fetchedCredential !== null ) {
                console.log("Fetched Credential", fetchedCredential);
                await api.post("/users/session", {
                    counter: counter,
                    pk: {
                        id: fetchedCredential.id,
                        // @ts-ignore
                        rawId: bufferToBase64url(fetchedCredential.rawId),
                        type: fetchedCredential.type,
                        response: {
                            // @ts-ignore
                            authenticatorData: bufferToBase64url(fetchedCredential.response.authenticatorData),
                            // @ts-ignore
                            clientDataJSON: bufferToBase64url(fetchedCredential.response.clientDataJSON),
                            // @ts-ignore
                            signature: bufferToBase64url(fetchedCredential.response.signature),
                            // @ts-ignores
                            userHandle: bufferToBase64url(fetchedCredential.response.userHandle)
                        },
                    }
                });
            }
        }
    };

    return (
        <div className="container mx-auto mt-12">
            <NavBar/>

            <div className="m-8">
                <div className="flex items-center space-x-4">
                    <Avatar className="h-16 w-16">
                        <AvatarImage src={basicCV} />
                    </Avatar>
                    <div>
                        <p className="text-lg">基本信息</p>
                        <p className="text-sm text-muted-foreground">
                            账号 & 邮箱等
                        </p>
                    </div>
                </div>

                <div className="m-4">
                    <div className="flex justify-between items-center my-2">
                        <p>用户名</p>
                        <p>{user?.username}</p>
                    </div>
                    <div className="flex justify-between items-center my-2">
                        <p>邮箱</p>
                        <p>{user?.email}</p>
                    </div>
                    <div className="flex justify-between items-center my-2">
                        <p>用户权限</p>
                        <p>{user?.roles}</p>
                    </div>

                    <div className="flex justify-between items-center my-2">
                        <p>passkey</p>
                        <div className="space-x-2">
                            <Button onClick={onCreatePasskey}>create</Button>
                            <Button onClick={onPasskeyAuthenticate}>login</Button>
                        </div>
                    </div>
                </div>
                <div className="border-t border-dashed border-gray-200 my-4"></div>
            </div>

            <div className="m-8">
                <div className="flex items-center space-x-4">
                    <Avatar className="h-16 w-16">
                        <AvatarImage src={dataPrivacy} />
                    </Avatar>
                    <div>
                        <p className="text-lg">数据 & 账号</p>
                        <p className="text-sm text-muted-foreground">
                            管理、导出你的数据和账号移除
                        </p>
                    </div>
                </div>
                <div className="m-4">
                    <div className="flex justify-between items-center my-2">
                        <p>导出所有数据</p>
                        <Button variant="outline" onClick={onDataExport}>
                            下载
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    );
}
