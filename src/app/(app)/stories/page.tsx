import { Button } from '~/components/Button';

// Mock data - replace with real data later
const mockStories = {
  featured: [
    {
      id: 1,
      title: "The Last City",
      author: "Sarah Chen",
      description: "A cyberpunk adventure where your choices determine the fate of humanity's final refuge.",
      coverImage: "https://images.unsplash.com/photo-1743448748313-80eb7f9eb2b7?w=400&h=300&fit=crop",
      rating: 4.8,
      readCount: 15420,
      tags: ["Cyberpunk", "Adventure", "Sci-Fi"],
    },
    {
      id: 2,
      title: "Whispers of the Forest",
      author: "Marcus Rodriguez",
      description: "A magical mystery where ancient secrets lie hidden in an enchanted woodland.",
      coverImage: "https://images.unsplash.com/photo-1441974231531-c6227db76b6e?w=400&h=300&fit=crop",
      rating: 4.9,
      readCount: 12850,
      tags: ["Fantasy", "Mystery", "Magic"],
    },
    {
      id: 3,
      title: "The Detective's Dilemma",
      author: "Emma Thompson",
      description: "Solve a complex murder mystery in 1920s London with branching storylines.",
      coverImage: "https://images.unsplash.com/photo-1481627834876-b7833e8f5570?w=400&h=300&fit=crop",
      rating: 4.7,
      readCount: 9870,
      tags: ["Mystery", "Historical", "Crime"],
    },
  ],
  trending: [
    {
      id: 4,
      title: "Digital Dreams",
      author: "Alex Kim",
      description: "Navigate a virtual reality world where dreams and technology collide.",
      coverImage: "https://images.unsplash.com/photo-1743448748313-80eb7f9eb2b7?w=400&h=300&fit=crop",
      rating: 4.6,
      readCount: 2340,
      tags: ["Sci-Fi", "Virtual Reality", "Thriller"],
    },
    {
      id: 5,
      title: "The Royal Heir",
      author: "Isabella Santos",
      description: "A medieval romance where you must navigate court politics and forbidden love.",
      coverImage: "https://images.unsplash.com/photo-1441974231531-c6227db76b6e?w=400&h=300&fit=crop",
      rating: 4.5,
      readCount: 1890,
      tags: ["Romance", "Historical", "Drama"],
    },
    {
      id: 6,
      title: "Quantum Quest",
      author: "David Park",
      description: "Explore parallel universes in this mind-bending science fiction adventure.",
      coverImage: "https://images.unsplash.com/photo-1481627834876-b7833e8f5570?w=400&h=300&fit=crop",
      rating: 4.4,
      readCount: 1560,
      tags: ["Sci-Fi", "Adventure", "Quantum"],
    },
  ],
  newest: [
    {
      id: 7,
      title: "The Artisan's Touch",
      author: "Maya Patel",
      description: "A cozy crafting story where you build relationships while mastering ancient arts.",
      coverImage: "https://images.unsplash.com/photo-1743448748313-80eb7f9eb2b7?w=400&h=300&fit=crop",
      rating: 4.3,
      readCount: 420,
      tags: ["Slice of Life", "Crafting", "Cozy"],
    },
    {
      id: 8,
      title: "Neon Nights",
      author: "Carlos Mendez",
      description: "A noir detective story set in a futuristic city of endless rain and neon lights.",
      coverImage: "https://images.unsplash.com/photo-1441974231531-c6227db76b6e?w=400&h=300&fit=crop",
      rating: 4.2,
      readCount: 380,
      tags: ["Noir", "Cyberpunk", "Detective"],
    },
    {
      id: 9,
      title: "The Lost Library",
      author: "Sophie Anderson",
      description: "Discover ancient knowledge in a magical library that exists between worlds.",
      coverImage: "https://images.unsplash.com/photo-1481627834876-b7833e8f5570?w=400&h=300&fit=crop",
      rating: 4.1,
      readCount: 290,
      tags: ["Fantasy", "Adventure", "Mystery"],
    },
  ],
};

function StoryCard({ story }: { story: typeof mockStories.featured[0] }) {
  return (
    <div className="group relative bg-white dark:bg-slate-800 rounded-xl shadow-lg hover:shadow-xl transition-all duration-300 overflow-hidden transform hover:-translate-y-1">
      <div className="aspect-[4/3] overflow-hidden">
        <img
          src={story.coverImage}
          alt={story.title}
          className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
        />
        <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
      </div>

      <div className="p-6">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
            {story.title}
          </h3>
          <div className="flex items-center gap-1">
            <span className="text-yellow-500">★</span>
            <span className="text-sm text-gray-600 dark:text-gray-400">{story.rating}</span>
          </div>
        </div>

        <p className="text-sm text-gray-600 dark:text-gray-400 mb-3 line-clamp-2">
          by {story.author}
        </p>

        <p className="text-sm text-gray-700 dark:text-gray-300 mb-4 line-clamp-3">
          {story.description}
        </p>

        <div className="flex items-center justify-between">
          <div className="flex flex-wrap gap-1">
            {story.tags.slice(0, 2).map((tag) => (
              <span
                key={tag}
                className="px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded-full"
              >
                {tag}
              </span>
            ))}
          </div>
          <span className="text-xs text-gray-500 dark:text-gray-400">
            {story.readCount.toLocaleString()} reads
          </span>
        </div>
      </div>

      <Button
        type="anchor"
        href={`/stories/${story.id}`}
        variant="text"
        color="primary"
        className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300"
      >
        <span className="sr-only">Read {story.title}</span>
      </Button>
    </div>
  );
}

function StorySection({ title, description, stories }: {
  title: string;
  description: string;
  stories: typeof mockStories.featured;
}) {
  return (
    <section className="py-16">
      <div className="mx-auto max-w-7xl px-6 lg:px-8">
        <div className="mx-auto max-w-2xl text-center mb-12">
          <h2 className="text-3xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-4xl">
            {title}
          </h2>
          <p className="mt-4 text-lg text-gray-600 dark:text-gray-300">
            {description}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
          {stories.map((story) => (
            <StoryCard key={story.id} story={story} />
          ))}
        </div>
      </div>
    </section>
  );
}

export default function StoriesPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 via-blue-50 to-indigo-100 dark:from-slate-900 dark:via-slate-800 dark:to-slate-900">
      {/* Hero Section */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 bg-grid-pattern opacity-5"></div>
        <div className="absolute top-0 left-1/4 w-72 h-72 bg-blue-400 rounded-full mix-blend-multiply filter blur-xl opacity-20 animate-blob"></div>
        <div className="absolute top-0 right-1/4 w-72 h-72 bg-purple-400 rounded-full mix-blend-multiply filter blur-xl opacity-20 animate-blob animation-delay-2000"></div>

        <div className="relative px-6 py-16 mx-auto max-w-7xl lg:px-8">
          <div className="mx-auto max-w-2xl text-center">
            <h1 className="text-4xl font-bold tracking-tight text-gray-900 dark:text-white sm:text-6xl">
              Discover Stories
            </h1>
            <p className="mt-6 text-lg leading-8 text-gray-600 dark:text-gray-300 max-w-2xl mx-auto">
              Immerse yourself in interactive narratives where every choice shapes your journey.
              From epic adventures to intimate dramas, find your next favorite story.
            </p>
          </div>
        </div>
      </section>

      {/* Featured Stories */}
      <StorySection
        title="Featured Stories"
        description="Handpicked stories that showcase the best of interactive fiction"
        stories={mockStories.featured}
      />

      {/* Trending Stories */}
      <StorySection
        title="Trending Now"
        description="Stories that are capturing readers' imaginations this week"
        stories={mockStories.trending}
      />

      {/* Newest Stories */}
      <StorySection
        title="Fresh Releases"
        description="The latest stories from our community of creators"
        stories={mockStories.newest}
      />
    </div>
  );
}
