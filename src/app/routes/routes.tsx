import { createBrowserRouter } from "react-router-dom";
import { AppLayout } from "../layouts/app-layout";
import { DashboardPage } from "@/modules/dashboard/dashboard-page";
import { InventoryPage } from "@/modules/inventory/inventory-page";
import { PosPage } from "@/modules/pos/pos-page";
import { SalesHistoryPage } from "@/modules/sales/sales-history-page";
import { PurchasesPage } from "@/modules/purchases/purchases-page";
import { PurchaseHistoryPage } from "@/modules/purchases/purchase-history-page";
import { ProductsPage } from "@/modules/products/products-page";
import { BusinessSetupPage } from "@/modules/settings/business-setup-page";
import { SuppliersPage } from "@/modules/suppliers/suppliers-page";
import { CustomersPage } from "@/modules/customers/customers-page";
import { PaymentsPage } from "@/modules/payments/payments-page";
import { ExpensesPage } from "@/modules/expenses/expenses-page";
import { ReportsPage } from "@/modules/reports/reports-page";
import { CashAccountsPage } from "@/modules/cash-accounts/cash-accounts-page";
import { EmployeesPage } from "@/modules/employees/employees-page";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppLayout />,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: "pos", element: <PosPage /> },
      { path: "products", element: <ProductsPage /> },
      { path: "inventory", element: <InventoryPage /> },
      { path: "purchases", element: <PurchasesPage /> },
      { path: "purchase-history", element: <PurchaseHistoryPage /> },
      { path: "sales", element: <SalesHistoryPage /> },
      { path: "customers", element: <CustomersPage /> },
      { path: "suppliers", element: <SuppliersPage /> },
      { path: "expenses", element: <ExpensesPage /> },
      { path: "payments", element: <PaymentsPage /> },
      { path: "cash-accounts", element: <CashAccountsPage /> },
      { path: "reports", element: <ReportsPage /> },
      { path: "employees", element: <EmployeesPage /> },
      { path: "settings", element: <BusinessSetupPage /> },
    ],
  },
]);
