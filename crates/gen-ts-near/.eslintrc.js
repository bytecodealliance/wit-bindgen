module.exports = {
    "env": {
        "browser": true,
        "es2021": true,
        "node": true
    },
    "extends": "eslint:recommended",
    "parserOptions": {
        "ecmaVersion": 12,
        "sourceType": "module"
    },
    "rules": {
      // allow this since we generate `const {} = e;` for empty structs
      "no-empty-pattern": 0,
      // TODO: we generate some unused functions by accident, let's fix that later
      "no-unused-vars": 0,
    }
};
