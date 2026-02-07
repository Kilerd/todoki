import { Button } from "@/components/ui/button";
import { Link } from "react-router-dom";
import useUserSession from "../hooks/useUserSession";
import { RotateCcw, LayoutList, ThumbsUp, Webhook, LayoutTemplate } from "lucide-react";

interface FeatureProps extends React.ComponentPropsWithoutRef<'div'> {
    icon: React.FC<any>;
    title: string;
    description: string;
}

function Feature({icon: Icon, title, description, className, ...others}: FeatureProps) {
    return (
        <div className={`relative py-10 flex flex-col items-baseline ${className}`} {...others}>
            <div className=" rounded inline-block p-10 pr-20 bg-primary/10 z-10" >
                <Icon className="w-10 h-10 text-primary" strokeWidth={1.5} />
            </div>

            <div className="absolute left-20 z-20 top-20 mt-7 flex flex-col items-baseline">
                
                <h3 className="font-bold text-lg mt-5 mb-2">
                    {title}
                </h3>
                <p className="text-sm text-muted-foreground">
                    {description}
                </p>
            </div>
        </div>
    );
}

const mockdata = [
    {
        icon: RotateCcw,
        title: '基于状态机的待办事项',
        description:
            '相比于传统的待办清单，Toodoo 提供了可以在多个状态之间流转的模式，可以更轻松地管理更加复杂、流程更长的事项。',
    },
    {
        icon: LayoutTemplate,
        title: '模版：别重复自己', 
        description:
            '足够实用的模版系统让 Toodoo 在新建待办事项的时候变得更加简单、更加直观。',
    },
    {
        icon: LayoutList,
        title: '时间线：回顾你的成就',
        description:
            '按照完成日期排序的时间线，在回顾过往待办事项时更自在，让站会的陈述面面俱到、一点不漏。',
    },
    {
        icon: ThumbsUp,
        title: '习惯：变成更好的自己',
        description:
            '内置的习惯系统可以方便地自动创建、追踪每日重复性任务，让学习追踪、健身追踪等习惯更加轻松、自在。',
    },
    {
        icon: Webhook,
        title: '开发者友好型',
        description:
            'Toodoo 提供了完整的数据导出、账号删除等隐私性完整控制。',
    },
];

function Homepage() {
    const {isAuthenticated, user, loadingUserData} = useUserSession()
    const isAuthed = !loadingUserData && isAuthenticated;
    const items = mockdata.map((item) => <Feature {...item} key={item.title}/>);

    return (
        <div>
            <div className="container mx-auto mt-16">
                <div className="flex flex-col items-baseline">
                    <h1 className=" inline-block text-5xl font-extrabold leading-tight mb-4">
                        {isAuthed && 
                            <>
                                <span className="text-primary">{user?.username}</span>
                                <span>, 欢迎回到 </span>
                            </>
                        }
                        <span className="bg-primary/10 rounded-sm px-3 py-1">Toodoo</span>
                    </h1>
                    <h1 className="text-5xl font-extrabold leading-tight">
                        直观、易用，且强大的
                        <span className="relative bg-primary/10 rounded-sm px-3 py-1 ml-2">待办清单</span>
                    </h1>

                    <div className="flex gap-4 mt-16">
                        <Button asChild size="lg">
                            <Link to={isAuthed ? "/tasks" : "/login"}>
                                {isAuthed ? "前往今日待办" : "已有账号，登录"}
                            </Link>
                        </Button>
                        {!isAuthed &&
                            <Button variant="outline" size="lg" asChild>
                                <Link to="/login?type=register">
                                    前往注册
                                </Link>
                            </Button>
                        }
                    </div>
                </div>

                <div className="grid md:grid-cols-2 gap-12 mt-24">
                    {items}
                </div>
            </div>
        </div>
    )
}

export default Homepage
