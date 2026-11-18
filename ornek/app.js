const button = document.querySelector('button');
const counter = document.querySelector('output');

button.addEventListener('click', () => {
  counter.value = String(Number(counter.value) + 1);
});
