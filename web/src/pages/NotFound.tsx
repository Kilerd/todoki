export default function NotFoundTitle() {
    return (
        <div className="container py-20">
            <div className="text-center font-black text-[220px] leading-none text-gray-200 dark:text-gray-800 mb-8 sm:text-[120px]">
                404
            </div>
            <h1 className="text-center font-black text-4xl sm:text-3xl mb-6">
                You have found a secret place.
            </h1>
            <p className="text-center text-lg text-muted-foreground max-w-[500px] mx-auto">
                Unfortunately, this is only a 404 page. You may have mistyped the address, or the page has
                been moved to another URL.
            </p>
        </div>
    );
}