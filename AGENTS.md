## 项目倾向与细节
- 后端采用了 gotcha + conservator的组合，需要把常规的业务接口接入openapi的系统内
- 前端尽可能的不自己实现接口的请求和response类型，需要用 npm run api 来从后端的openapi端口进行自动生成
- 前端需要保证 npm run build 通过