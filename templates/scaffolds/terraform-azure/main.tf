terraform {
  required_version = ">= 1.7.0"

  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 4.0"
    }
  }

  backend "azurerm" {
    resource_group_name  = "rg-ops-ecc-tfstate"
    storage_account_name = "stfstateopseccprod"
    container_name       = "tfstate"
    key                  = "prod.terraform.tfstate"
  }
}

provider "azurerm" {
  features {
    key_vault {
      purge_soft_delete_on_destroy = false
    }
  }
}

variable "location" {
  type        = string
  description = "Primary Azure region"
  default     = "eastus2"
}

variable "project_name" {
  type        = string
  description = "Project prefix"
  default     = "opsecc"
}

variable "sql_admin_login" {
  type        = string
  description = "Azure SQL Entra/Admin login username"
  default     = "sqladminuser"
}

variable "sql_admin_password" {
  type        = string
  description = "Azure SQL administrator password injected from Key Vault / TF_VAR"
  sensitive   = true
}

resource "azurerm_resource_group" "main" {
  name     = "rg-${var.project_name}-prod"
  location = var.location
}

resource "azurerm_virtual_network" "main" {
  name                = "vnet-${var.project_name}-prod"
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name
  address_space       = ["10.40.0.0/16"]
}

resource "azurerm_subnet" "apps" {
  name                 = "snet-container-apps"
  resource_group_name  = azurerm_resource_group.main.name
  virtual_network_name = azurerm_virtual_network.main.name
  address_prefixes     = ["10.40.1.0/24"]
}

resource "azurerm_subnet" "data" {
  name                 = "snet-data-isolated"
  resource_group_name  = azurerm_resource_group.main.name
  virtual_network_name = azurerm_virtual_network.main.name
  address_prefixes     = ["10.40.10.0/24"]
}

resource "azurerm_network_security_group" "apps" {
  name                = "nsg-${var.project_name}-apps"
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name

  security_rule {
    name                       = "AllowHTTPSInternal"
    priority                   = 100
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "443"
    source_address_prefix      = "10.40.0.0/16"
    destination_address_prefix = "*"
  }
}

resource "azurerm_storage_account" "artifacts" {
  name                            = "st${var.project_name}prod001"
  resource_group_name             = azurerm_resource_group.main.name
  location                        = azurerm_resource_group.main.location
  account_tier                    = "Standard"
  account_replication_type        = "ZRS"
  min_tls_version                 = "TLS1_2"
  https_traffic_only_enabled      = true
  allow_nested_items_to_be_public = false
  public_network_access_enabled   = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "azurerm_mssql_server" "sql" {
  name                          = "sql-${var.project_name}-prod"
  resource_group_name           = azurerm_resource_group.main.name
  location                      = azurerm_resource_group.main.location
  version                       = "12.0"
  administrator_login           = var.sql_admin_login
  administrator_login_password  = var.sql_admin_password
  minimum_tls_version           = "1.2"
  public_network_access_enabled = false

  lifecycle {
    prevent_destroy = true
  }
}

resource "azurerm_mssql_database" "app_db" {
  name           = "sqldb-${var.project_name}-prod"
  server_id      = azurerm_mssql_server.sql.id
  sku_name       = "S1"
  zone_redundant = true

  lifecycle {
    prevent_destroy = true
  }
}
