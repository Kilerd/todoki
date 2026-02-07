import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useToast } from "@/hooks/use-toast";
import { AxiosError } from "axios";
import { FormEvent, useState } from 'react';
import useUserSession from "../hooks/useUserSession";
import { api } from "../services/api";

interface FormValues {
    email: string;
    username: string;
    password: string;
}

function Login() {
    const { signIn } = useUserSession()
    const [type, setType] = useState<'login' | 'register'>('login');
    const { toast } = useToast()
    const [formValues, setFormValues] = useState<FormValues>({
        email: '',
        username: '',
        password: '',
    });
    const [errors, setErrors] = useState<Partial<FormValues>>({});

    const validate = (values: FormValues) => {
        const errors: Partial<FormValues> = {};
        
        if (type === 'register' && values.username.length <= 2) {
            errors.username = "Username should be at least 3 characters";
        }
        if (!/^\S+@\S+$/.test(values.email)) {
            errors.email = 'Invalid email';
        }
        if (values.password.length < 6) {
            errors.password = 'Password should include at least 6 characters';
        }
        
        return errors;
    };

    const handleInputChange = (field: keyof FormValues, value: string) => {
        setFormValues(prev => ({
            ...prev,
            [field]: value
        }));
    };

    async function handleSubmit(e: FormEvent) {
        e.preventDefault();
        
        const validationErrors = validate(formValues);
        setErrors(validationErrors);
        
        if (Object.keys(validationErrors).length > 0) {
            return;
        }

        if (type === "login") {
            try {
                await signIn({email: formValues.email, password: formValues.password})
            } catch (error) {
                const err = error as AxiosError
                toast({
                    title: "登录失败",
                    description: err.response?.data?.toString() ?? "",
                    variant: "destructive"
                });
            }
        }

        if (type === "register") {
            try {
                const response = await api.post('/users', {
                    username: formValues.username,
                    email: formValues.email,
                    password: formValues.password
                })
                if (response.status === 200) {
                    setType('login')
                    toast({
                        title: "注册成功🎉🎉🎉",
                        description: "现在可以登录了",
                    });
                } else {
                    toast({
                        title: "注册失败",
                        description: response.data,
                    });
                }
            } catch (error) {
                const err = error as AxiosError
                toast({
                    title: "注册失败",
                    description: err.response?.data?.toString() ?? "",
                    variant: "destructive"
                });
            }
        }
    }

    return (
        <div className="container mx-auto">
            <div className="min-h-screen flex items-center justify-center">
                <div className="w-full p-8 border rounded-lg shadow-sm">
                    <h2 className="text-2xl font-bold text-center mb-16">
                        {type === 'register' ? '注册' : "登录"} Toodoo.top
                    </h2>
                    
                    <form onSubmit={handleSubmit} className="space-y-6">
                        {type === 'register' && (
                            <div className="space-y-2">
                                <Input
                                    required
                                    placeholder="Your name"
                                    value={formValues.username}
                                    onChange={(e) => handleInputChange('username', e.target.value)}
                                    className={errors.username ? "border-red-500" : ""}
                                />
                                {errors.username && (
                                    <p className="text-sm text-red-500">{errors.username}</p>
                                )}
                            </div>
                        )}

                        <div className="space-y-2">
                            <Input
                                required
                                type="email"
                                placeholder="Your email"
                                value={formValues.email}
                                onChange={(e) => handleInputChange('email', e.target.value)}
                                className={errors.email ? "border-red-500" : ""}
                            />
                            {errors.email && (
                                <p className="text-sm text-red-500">{errors.email}</p>
                            )}
                        </div>

                        <div className="space-y-2">
                            <Input
                                required
                                type="password"
                                placeholder="Your password"
                                value={formValues.password}
                                onChange={(e) => handleInputChange('password', e.target.value)}
                                className={errors.password ? "border-red-500" : ""}
                            />
                            {errors.password && (
                                <p className="text-sm text-red-500">{errors.password}</p>
                            )}
                        </div>

                        <div className="flex justify-between items-center mt-12">
                            <button
                                type="button"
                                onClick={() => setType(type === 'register' ? 'login' : 'register')}
                                className="text-sm text-muted-foreground hover:underline"
                            >
                                {type === 'register'
                                    ? '已经注册？ 前往登录'
                                    : "注册新账号"}
                            </button>
                            
                            <Button type="submit" size="lg" className="rounded-full">
                                {type === 'login' ? "登录" : "提交注册"}
                            </Button>
                        </div>
                    </form>
                </div>
            </div>
        </div>
    )
}

export default Login
