import os
import pickle
import re
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

try:
    import numpy as np
    from sklearn.feature_extraction.text import TfidfVectorizer
    from sklearn.naive_bayes import ComplementNB
    from sklearn.pipeline import Pipeline
    SKLEARN_AVAILABLE = True
except ImportError:
    np = None
    TfidfVectorizer = None
    ComplementNB = None
    Pipeline = None
    SKLEARN_AVAILABLE = False

MODEL_DIR = Path(__file__).resolve().parent.parent.parent / "data"
MODEL_FILE = MODEL_DIR / "classifier.pkl"


class DocumentClassifier:
    """Scikit-learn document classifier for auto-matching tags, correspondents, and document types."""

    def __init__(self):
        self.vectorizer: Optional[TfidfVectorizer] = None
        self.tags_classifiers: Dict[int, ComplementNB] = {}
        self.correspondent_classifier: Optional[ComplementNB] = None
        self.doc_type_classifier: Optional[ComplementNB] = None
        self.is_trained = False
        self.load()

    def load(self) -> bool:
        if MODEL_FILE.exists():
            try:
                with open(MODEL_FILE, "rb") as f:
                    data = pickle.load(f)
                    self.vectorizer = data.get("vectorizer")
                    self.tags_classifiers = data.get("tags_classifiers", {})
                    self.correspondent_classifier = data.get("correspondent_classifier")
                    self.doc_type_classifier = data.get("doc_type_classifier")
                    self.is_trained = data.get("is_trained", False)
                return True
            except Exception as e:
                print(f"Failed to load classifier model: {e}")
        return False

    def save(self):
        MODEL_DIR.mkdir(parents=True, exist_ok=True)
        with open(MODEL_FILE, "wb") as f:
            pickle.dump({
                "vectorizer": self.vectorizer,
                "tags_classifiers": self.tags_classifiers,
                "correspondent_classifier": self.correspondent_classifier,
                "doc_type_classifier": self.doc_type_classifier,
                "is_trained": self.is_trained,
            }, f)

    def train(self, documents: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Train classifier using documents list:
        [{
            "content": str,
            "tags": [int],
            "correspondent_id": int | None,
            "document_type_id": int | None
        }]
        """
        if not SKLEARN_AVAILABLE:
            return {"status": "skipped", "reason": "scikit-learn is not installed; keyword matching is active"}

        if len(documents) < 2:
            return {"status": "skipped", "reason": "Not enough training documents (< 2)"}

        corpus = [doc.get("content", "") for doc in documents]
        self.vectorizer = TfidfVectorizer(
            ngram_range=(1, 2),
            max_features=10000,
            sublinear_tf=True,
            stop_words="english",
        )
        X = self.vectorizer.fit_transform(corpus)

        # Train binary classifier for each tag
        tag_sets = [set(doc.get("tags", [])) for doc in documents]
        all_tags = set().union(*tag_sets)
        self.tags_classifiers = {}

        for tag_id in all_tags:
            y = np.array([1 if tag_id in tags else 0 for tags in tag_sets])
            if len(np.unique(y)) > 1:
                clf = ComplementNB()
                clf.fit(X, y)
                self.tags_classifiers[tag_id] = clf

        # Train correspondent classifier
        corr_docs = [(i, doc.get("correspondent_id")) for i, doc in enumerate(documents) if doc.get("correspondent_id") is not None]
        if corr_docs:
            y_corr = [c[1] for c in corr_docs]
            if len(np.unique(y_corr)) > 1:
                X_corr = X[[c[0] for c in corr_docs]]
                self.correspondent_classifier = ComplementNB()
                self.correspondent_classifier.fit(X_corr, y_corr)

        # Train document type classifier
        type_docs = [(i, doc.get("document_type_id")) for i, doc in enumerate(documents) if doc.get("document_type_id") is not None]
        if type_docs:
            y_type = [t[1] for t in type_docs]
            if len(np.unique(y_type)) > 1:
                X_type = X[[t[0] for t in type_docs]]
                self.doc_type_classifier = ComplementNB()
                self.doc_type_classifier.fit(X_type, y_type)

        self.is_trained = True
        self.save()
        return {
            "status": "trained",
            "document_count": len(documents),
            "tag_models": len(self.tags_classifiers),
        }

    def predict(self, content: str, threshold: float = 0.5) -> Dict[str, Any]:
        """Predict tags, correspondent, and document type with probability threshold."""
        suggestions: Dict[str, Any] = {
            "tags": [],
            "correspondent_id": None,
            "document_type_id": None,
        }

        # Rule/keyword-based heuristic fallback
        lower = content.lower()
        if "invoice" in lower or "bill" in lower:
            suggestions["tags"].append("Invoice")
        if "receipt" in lower:
            suggestions["tags"].append("Receipt")
        if "statement" in lower or "bank" in lower:
            suggestions["tags"].append("Bank")
        if "contract" in lower or "agreement" in lower:
            suggestions["tags"].append("Contract")
        if "tax" in lower or "w2" in lower or "1099" in lower:
            suggestions["tags"].append("Tax")

        # Scikit-learn predictions if trained
        if self.is_trained and self.vectorizer and content.strip():
            try:
                X = self.vectorizer.transform([content])

                # Tags
                for tag_id, clf in self.tags_classifiers.items():
                    probas = clf.predict_proba(X)[0]
                    # Index 1 is positive class
                    if len(probas) > 1 and probas[1] >= threshold:
                        if tag_id not in suggestions["tags"]:
                            suggestions["tags"].append(tag_id)

                # Correspondent
                if self.correspondent_classifier:
                    probas = self.correspondent_classifier.predict_proba(X)[0]
                    best_idx = np.argmax(probas)
                    if probas[best_idx] >= threshold:
                        suggestions["correspondent_id"] = int(self.correspondent_classifier.classes_[best_idx])

                # Document Type
                if self.doc_type_classifier:
                    probas = self.doc_type_classifier.predict_proba(X)[0]
                    best_idx = np.argmax(probas)
                    if probas[best_idx] >= threshold:
                        suggestions["document_type_id"] = int(self.doc_type_classifier.classes_[best_idx])
            except Exception as e:
                print(f"Prediction failed with model: {e}")

        return suggestions


classifier_service = DocumentClassifier()
