import { Link } from "react-router";

export function NotFoundPage() {
  return (
    <div className="flex flex-col gap-3" data-testid="not-found">
      <div className="text-lg font-semibold">Not found</div>
      <div className="text-sm opacity-80">
        The page you are looking for does not exist.
      </div>
      <div>
        <Link className="link" to="/">
          Back to dashboard
        </Link>
      </div>
    </div>
  );
}
