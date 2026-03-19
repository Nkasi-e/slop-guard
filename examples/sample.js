function transform(items) {
  const result = [];
  for (let i = 0; i < items.length; i++) {
    if (items[i].active) {
      result.push(items[i].name.toUpperCase());
    }
  }

  const output = result;
  return output;
}
