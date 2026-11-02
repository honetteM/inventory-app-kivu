import { RouterProvider } from "react-router-dom";
import { AppProviders } from "./app/providers/app-providers";
import { router } from "./app/routes/routes";
import { AuthGate } from "./modules/auth/auth-gate";

export default function App() {
  return (
    <AppProviders>
      <AuthGate>
        <RouterProvider router={router} />
      </AuthGate>
    </AppProviders>
  );
}
