import { RouterProvider } from "react-router-dom";
import { AppProviders } from "./app/providers/app-providers";
import { router } from "./app/routes/routes";

export default function App() {
  return (
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  );
}
