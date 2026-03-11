document.addEventListener("DOMContentLoaded", () => {
  const filterButtons = Array.from(document.querySelectorAll("[data-filter]"));
  const taskCards = Array.from(document.querySelectorAll("[data-task-card]"));

  if (filterButtons.length > 0 && taskCards.length > 0) {
    const taskTitle = document.getElementById("task-title");
    const taskStatus = document.getElementById("task-status");
    const taskOwner = document.getElementById("task-owner");
    const taskDescription = document.getElementById("task-description");

    const applyFilter = (value) => {
      taskCards.forEach((card) => {
        const visible = value === "all" || card.dataset.status === value;
        card.classList.toggle("hidden", !visible);
      });

      const firstVisible = taskCards.find((card) => !card.classList.contains("hidden"));
      if (firstVisible) {
        selectTask(firstVisible);
      }
    };

    const selectTask = (card) => {
      taskCards.forEach((item) => item.classList.remove("is-active"));
      card.classList.add("is-active");
      taskTitle.textContent = card.dataset.title;
      taskStatus.textContent = `Статус: ${card.dataset.statusLabel}`;
      taskOwner.textContent = card.dataset.owner;
      taskDescription.textContent = card.dataset.description;
    };

    filterButtons.forEach((button) => {
      button.addEventListener("click", () => {
        filterButtons.forEach((item) => item.classList.remove("is-active"));
        button.classList.add("is-active");
        applyFilter(button.dataset.filter);
      });
    });

    taskCards.forEach((card) => {
      card.addEventListener("click", () => selectTask(card));
    });

    applyFilter("all");
  }

  const docButtons = Array.from(document.querySelectorAll("[data-doc-button]"));
  if (docButtons.length > 0) {
    const docTitle = document.getElementById("doc-title");
    const docText = document.getElementById("doc-text");
    const docHint = document.getElementById("doc-hint");

    docButtons.forEach((button) => {
      button.addEventListener("click", () => {
        docButtons.forEach((item) => item.classList.remove("is-active"));
        button.classList.add("is-active");
        docTitle.textContent = button.dataset.docTitle;
        docText.textContent = button.dataset.docText;
        docHint.textContent = button.dataset.docHint;
      });
    });
  }
});