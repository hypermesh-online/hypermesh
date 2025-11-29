#!/bin/bash

# Fix core imports in assets module
echo "Fixing import paths..."

# In assets/core files, imports should be relative or use super
find src/assets/core -name "*.rs" -exec sed -i 's/use crate::core::/use super::/g' {} \;

# In assets/adapters files, imports should point to assets::core
find src/assets/adapters -name "*.rs" -exec sed -i 's/use crate::core::/use crate::assets::core::/g' {} \;

# In other assets files, imports should point to assets::core
find src/assets -name "*.rs" -not -path "src/assets/core/*" -not -path "src/assets/adapters/*" \
    -exec sed -i 's/use crate::core::/use crate::assets::core::/g' {} \;

# Fix proxy imports
find src/assets -name "*.rs" -exec sed -i 's/use crate::proxy::/use crate::assets::proxy::/g' {} \;

# Fix extensions manager import
sed -i 's/use crate::extensions::manager/use crate::extensions/' src/api/extensions.rs

# Fix monitoring imports
sed -i 's/use crate::monitoring::/use crate::runtime::monitoring::/g' src/container/runtime.rs

echo "Import paths fixed!"